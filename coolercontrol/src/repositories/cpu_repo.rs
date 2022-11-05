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

use std::ops::{Deref, DerefMut};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{debug, error, info};
use psutil::cpu::CpuPercentCollector;
use psutil::sensors::TemperatureSensor;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::device::{ChannelStatus, Device, DeviceInfo, DeviceType, Status, TempStatus};
use crate::repositories::repository::Repository;
use crate::setting::Setting;

const CPU_TEMP_NAME: &str = "CPU Temp";
const CPU_LOAD_NAME: &str = "CPU Load";
const PSUTIL_CPU_SENSOR_NAMES: [&'static str; 4] =
    ["thinkpad", "k10temp", "coretemp", "zenpower"];
const PSUTIL_CPU_SENSOR_LABELS: [&'static str; 6] =
    ["CPU", "tctl", "physical", "package", "tdie", ""];

/// A CPU Repository for CPU status
pub struct CpuRepo {
    devices: RwLock<Vec<Device>>,
    cpu_collector: RwLock<CpuPercentCollector>,
    current_sensor_name: RwLock<Option<String>>,
    current_label_name: RwLock<Option<String>>,
}

impl CpuRepo {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            devices: RwLock::new(vec![]),
            cpu_collector: RwLock::new(CpuPercentCollector::new()?),
            current_sensor_name: RwLock::new(None),
            current_label_name: RwLock::new(None),
        })
    }

    async fn request_status(&self) -> Result<Status> {
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
        // todo: request_status_known(temp_sensors) if current_* is set -> for small speedup
        self.request_status_new(temp_sensors).await
    }

    /// This is used to find the correct sensors and labels for cpu data.
    async fn request_status_new(
        &self,
        temp_sensors: Vec<TemperatureSensor>,
    ) -> Result<Status> {
        for cpu_sensor_name in PSUTIL_CPU_SENSOR_NAMES {  // order is important
            for temp_sensor in &temp_sensors {
                if temp_sensor.unit() == cpu_sensor_name {
                    if let Some(sensor_label) = temp_sensor.label() {
                        let label = Self::sanitize_label(sensor_label);
                        for cpu_label in PSUTIL_CPU_SENSOR_LABELS {
                            if label.contains(cpu_label) {
                                self.set_current_sensor_names(cpu_sensor_name, &label).await;
                                let cpu_usage = self.cpu_collector.write().await.cpu_percent()?;
                                return Ok(Status {
                                    temps: vec![TempStatus {
                                        name: CPU_TEMP_NAME.to_string(),
                                        temp: temp_sensor.current().celsius(),
                                        frontend_name: CPU_TEMP_NAME.to_string(),
                                        external_name: CPU_TEMP_NAME.to_string(),
                                    }],
                                    channels: vec![ChannelStatus {
                                        name: CPU_LOAD_NAME.to_string(),
                                        rpm: None,
                                        duty: Some(cpu_usage as f64),
                                        pwm_mode: None,
                                    }],
                                    ..Default::default()
                                });
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
            .replace(label.to_string());
    }

    async fn request_status_known(&self, temp_sensors: Vec<TemperatureSensor>) -> Result<Status> {
        todo!()
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
    async fn initialize_devices(&self) -> Result<()> {
        // todo: handle multiple cpus
        debug!("Starting Device Initialization");
        let start_initialization = Instant::now();
        let status = self.request_status().await?;
        let cpu_name = self.get_cpu_name().await;
        let mut device = Device {
            name: cpu_name,
            d_type: DeviceType::CPU,
            type_id: 1,
            info: Some(DeviceInfo {
                temp_max: 100,
                temp_ext_available: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        device.set_status(status);
        self.devices.write().await.push(device);
        debug!("Initialized Devices: {:?}", self.devices.read().await);
        debug!(
            "Time taken to initialize all CPU devices: {:?}", start_initialization.elapsed()
        );
        info!("All CPU devices initialized");
        Ok(())
    }

    async fn devices(&self) -> Vec<Device> {
        let mut vec = vec![];
        for dev in self.devices.read().await.deref() {
            vec.push(dev.clone())  // Currently clones all devices
        }
        vec
    }

    async fn update_statuses(&self) -> Result<()> {
        debug!("Updating all CPU device statuses");
        let start_update = Instant::now();
        // current only supports one device:
        for device in self.devices.write().await.deref_mut() {
            let status = self.request_status().await?;
            debug!("Device status updatedL: {:?}", status);
            device.set_status(status);
        }
        debug!(
            "Time taken to get status for all CPU devices: {:?}",
            start_update.elapsed()
        );
        info!("All CPU device statuses updated");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.devices.write().await.clear();
        debug!("CPU Repo shutdown");
        Ok(())
    }

    async fn apply_setting(&self, device_type_id: u8, setting: Setting) -> Result<()> {
        Err(anyhow!("Applying settings is not supported for CPU devices"))
    }
}