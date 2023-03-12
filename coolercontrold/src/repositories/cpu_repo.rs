/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2022  Guy Boldon
 * |
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * |
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * |
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, error, info};
use psutil::cpu::CpuPercentCollector;
use psutil::sensors::TemperatureSensor;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;
use zbus::export::futures_util::future::join_all;

use crate::device::{ChannelStatus, Device, DeviceInfo, DeviceType, Status, TempStatus, UID};
use crate::repositories::repository::{DeviceList, Repository};
use crate::setting::Setting;

const CPU_TEMP_NAME: &str = "CPU Temp";
const CPU_LOAD_NAME: &str = "CPU Load";
pub const PSUTIL_CPU_SENSOR_NAMES: [&'static str; 4] =
    ["thinkpad", "k10temp", "coretemp", "zenpower"];
const PSUTIL_CPU_SENSOR_LABELS: [&'static str; 6] =
    ["CPU", "tctl", "physical", "package", "tdie", ""];

/// A CPU Repository for CPU status
pub struct CpuRepo {
    devices: DeviceList,
    cpu_collector: RwLock<CpuPercentCollector>,
    current_sensor_name: RwLock<Option<String>>,
    current_label_name: RwLock<Option<String>>,
    preloaded_statuses: RwLock<HashMap<u8, (Vec<ChannelStatus>, Vec<TempStatus>)>>,
}

impl CpuRepo {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            devices: vec![],
            cpu_collector: RwLock::new(CpuPercentCollector::new()?),
            current_sensor_name: RwLock::new(None),
            current_label_name: RwLock::new(None),
            preloaded_statuses: RwLock::new(HashMap::new()),
        })
    }

    async fn request_status(&self) -> Result<(Vec<ChannelStatus>, Vec<TempStatus>)> {
        let mut temp_sensors = vec![];
        for sensor_result in psutil::sensors::temperatures() {
            if let Ok(sensor) = sensor_result {
                temp_sensors.push(sensor)
            }
        }
        // let physical_cpu_count = psutil::cpu::cpu_count_physical();
        if self.current_sensor_name.read().await.is_none() {
            // only log all responses the first time
            debug!("Detected temperature sensors: {:?}", temp_sensors);
        }
        self.request_statuses_new(temp_sensors).await
    }

    /// This is used to find the correct sensors and labels for cpu data.
    async fn request_statuses_new(
        &self,
        temp_sensors: Vec<TemperatureSensor>,
    ) -> Result<(Vec<ChannelStatus>, Vec<TempStatus>)> {
        for cpu_sensor_name in PSUTIL_CPU_SENSOR_NAMES {  // order is important
            for temp_sensor in &temp_sensors {
                if temp_sensor.unit() == cpu_sensor_name {
                    if let Some(sensor_label) = temp_sensor.label() {
                        let label = Self::sanitize_label(sensor_label);
                        for cpu_label in PSUTIL_CPU_SENSOR_LABELS {  // order is important
                            if label.contains(cpu_label) {
                                self.set_current_sensor_names(cpu_sensor_name, &label).await;
                                let cpu_usage = self.cpu_collector.write().await.cpu_percent()?;
                                return Ok((
                                    vec![ChannelStatus {
                                        name: CPU_LOAD_NAME.to_string(),
                                        rpm: None,
                                        duty: Some(cpu_usage as f64),
                                        pwm_mode: None,
                                    }],
                                    vec![TempStatus {
                                        name: CPU_TEMP_NAME.to_string(),
                                        temp: temp_sensor.current().celsius(),
                                        frontend_name: CPU_TEMP_NAME.to_string(),
                                        external_name: CPU_TEMP_NAME.to_string(),
                                    }],
                                ));
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow!("No CPU Temperatures found: {:?}", temp_sensors))
    }

    fn sanitize_label(sensor_label: &str) -> String {
        sensor_label.to_lowercase().replace(" ", "_")
    }

    async fn set_current_sensor_names(&self, cpu_sensor_name: &str, label: &String) {
        self.current_sensor_name.write().await
            .replace(cpu_sensor_name.to_string());
        self.current_label_name.write().await
            .replace(label.clone());
    }

    async fn get_cpu_name(&self) -> String {
        let output = Command::new("sh")
            .arg("-c")
            .arg("LC_ALL=C lscpu")
            .output().await;
        match output {
            Ok(out) => {
                if out.status.success() {
                    let out_str = String::from_utf8(out.stdout).unwrap();
                    for line in out_str.trim().lines() {
                        if line.to_lowercase().contains("model name") {
                            let parts = line.split(":").collect::<Vec<&str>>();
                            if parts.len() > 1 {
                                return parts[1].trim().to_string();
                            }
                        }
                    }
                    error!("Looking up CPU name returned unexpected response:\n{}", out_str)
                } else {
                    let out_err = String::from_utf8(out.stderr).unwrap();
                    error!("Error looking up CPU name: {}", out_err)
                }
            }
            Err(err) => error!("Error looking up CPU name: {}", err)
        }
        "cpu".to_string()
    }
}

#[async_trait]
impl Repository for CpuRepo {
    fn device_type(&self) -> DeviceType {
        DeviceType::CPU
    }

    async fn initialize_devices(&mut self) -> Result<()> {
        // todo: handle multiple cpus
        //   To do this correctly, I see we just get more Tctl temperatures from the system, but
        //   to really properly track wich cpu socket belongs to which temp we need to handle
        //   the hwmon files ourselves. (device path aka UID)
        debug!("Starting Device Initialization");
        let type_index = 1u8;
        let start_initialization = Instant::now();
        let (channels, temps) = self.request_status().await?;
        self.preloaded_statuses.write().await
            .insert(type_index, (channels.clone(), temps.clone()));
        let status = Status {
            channels,
            temps,
            ..Default::default()
        };
        let cpu_name = self.get_cpu_name().await;
        let device = Device::new(
            cpu_name,
            DeviceType::CPU,
            type_index,
            None,
            Some(DeviceInfo {
                temp_max: 100,
                temp_ext_available: true,
                ..Default::default()
            }),
            Some(status),
            None,  // use default
        );
        self.devices.push(Arc::new(RwLock::new(device)));
        let mut init_devices = vec![];
        for device in self.devices.iter() {
            init_devices.push(device.read().await.clone())
        }
        if log::max_level() == log::LevelFilter::Debug {
            info!("Initialized Devices: {:#?}", init_devices);  // pretty output for easy reading
        } else {
            info!("Initialized Devices: {:?}", init_devices);
        }
        debug!(
            "Time taken to initialize all CPU devices: {:?}", start_initialization.elapsed()
        );
        info!("CPU Repository initialized");
        Ok(())
    }

    async fn devices(&self) -> DeviceList {
        self.devices.iter().cloned().collect()
    }

    async fn preload_statuses(&self) {
        let start_update = Instant::now();
        let mut futures = Vec::new();
        for device_lock in &self.devices {
            futures.push(
                async {
                    let status = self.request_status().await;
                    if let Err(err) = status {
                        error!("Error getting CPU status: {}", err);
                        return;
                    }
                    let device_id = device_lock.read().await.type_index;
                    self.preloaded_statuses.write().await.insert(device_id, status.unwrap());
                }
            )
        }
        join_all(futures).await;
        debug!(
            "STATUS PRELOAD Time taken for all CPU devices: {:?}",
            start_update.elapsed()
        );
    }

    async fn update_statuses(&self) -> Result<()> {
        let start_update = Instant::now();
        // current only supports one device:
        for device_lock in &self.devices {
            let preloaded_statuses_map = self.preloaded_statuses.read().await;
            let preloaded_statuses = preloaded_statuses_map.get(&device_lock.read().await.type_index);
            if let None = preloaded_statuses {
                error!("There is no status preloaded for this CPU device: {}", device_lock.read().await.type_index);
                continue;
            }
            let (channels, temps) = preloaded_statuses.unwrap().clone();
            let status = Status {
                channels,
                temps,
                ..Default::default()
            };
            debug!("Device status updated: {:?}", status);
            device_lock.write().await.set_status(status);
        }
        debug!(
            "STATUS SNAPSHOT Time taken for all CPU devices: {:?}",
            start_update.elapsed()
        );
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("CPU Repository shutdown");
        Ok(())
    }

    async fn apply_setting(&self, _device_uid: &UID, _setting: &Setting) -> Result<()> {
        Err(anyhow!("Applying settings is not supported for CPU devices"))
    }
}