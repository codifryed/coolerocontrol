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
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::device::{ChannelInfo, ChannelStatus, Device, DeviceInfo, DeviceType, SpeedOptions, Status, TempStatus};
use crate::repositories::hwmon::devices::DeviceFns;
use crate::repositories::hwmon::fans::FanFns;
use crate::repositories::hwmon::hwmon_repo::{HwmonChannelInfo, HwmonChannelType, HwmonDriverInfo};
use crate::repositories::hwmon::temps::TempFns;
use crate::repositories::repository::{DeviceList, DeviceLock, Repository};
use crate::setting::Setting;

const GPU_TEMP_NAME: &str = "GPU Temp";
const GPU_LOAD_NAME: &str = "GPU Load";
const GPU_FAN_NAME: &str = "GPU Fan";
const DEFAULT_AMD_GPU_NAME: &str = "Radeon Graphics";
const AMD_HWMON_NAME: &str = "amdgpu";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, Serialize, Deserialize)]
pub enum GpuType {
    Nvidia,
    AMD,
}

/// A Repository for GPU devices
pub struct GpuRepo {
    devices: HashMap<u8, DeviceLock>,
    amd_device_infos: HashMap<u8, HwmonDriverInfo>,
    gpu_type_count: RwLock<HashMap<GpuType, u8>>,
    has_multiple_gpus: RwLock<bool>,
}

impl GpuRepo {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            devices: HashMap::new(),
            amd_device_infos: HashMap::new(),
            gpu_type_count: RwLock::new(HashMap::new()),
            has_multiple_gpus: RwLock::new(false),
        })
    }

    async fn detect_gpu_types(&self) {
        {
            let mut type_count = self.gpu_type_count.write().await;
            type_count.insert(GpuType::Nvidia, self.get_nvidia_status().await.len() as u8);
            type_count.insert(GpuType::AMD, init_amd_devices().await.len() as u8);
        }
        let number_of_gpus = self.gpu_type_count.read().await.values().sum::<u8>();
        let mut has_multiple_gpus = self.has_multiple_gpus.write().await;
        *has_multiple_gpus = number_of_gpus > 1;
        if number_of_gpus == 0 {
            warn!("No GPU Devices detected")
        }
    }

    async fn request_statuses(&self) -> Vec<(Status, String)> {
        let mut statuses = vec![];
        if self.gpu_type_count.read().await.get(&GpuType::Nvidia).unwrap() > &0 {
            statuses.extend(
                self.request_nvidia_statuses().await
            )
        }
        statuses
    }

    async fn request_nvidia_statuses(&self) -> Vec<(Status, String)> {
        let has_multiple_gpus: bool = self.has_multiple_gpus.read().await.clone();
        let mut statuses = vec![];
        let nvidia_statuses = self.get_nvidia_status().await;
        let starting_gpu_index = if has_multiple_gpus {
            self.gpu_type_count.read().await.get(&GpuType::AMD).unwrap_or(&0) + 1
        } else {
            1
        };
        for (index, nvidia_status) in nvidia_statuses.iter().enumerate() {
            let index = index as u8;
            let mut temps = vec![];
            let mut channels = vec![];
            if let Some(temp) = nvidia_status.temp {
                let gpu_temp_name_prefix = if has_multiple_gpus {
                    format!("#{} ", starting_gpu_index + index)
                } else {
                    "".to_string()
                };
                temps.push(
                    TempStatus {
                        name: GPU_TEMP_NAME.to_string(),
                        temp,
                        frontend_name: GPU_TEMP_NAME.to_string(),
                        external_name: gpu_temp_name_prefix + GPU_TEMP_NAME,
                    }
                );
            }
            if let Some(load) = nvidia_status.load {
                channels.push(
                    ChannelStatus {
                        name: GPU_LOAD_NAME.to_string(),
                        rpm: None,
                        duty: Some(load as f64),
                        pwm_mode: None,
                    }
                );
            }
            if let Some(fan_duty) = nvidia_status.fan_duty {
                channels.push(
                    ChannelStatus {
                        name: GPU_FAN_NAME.to_string(),
                        rpm: None,
                        duty: Some(fan_duty as f64),
                        pwm_mode: None,
                    }
                )
            }
            statuses.push(
                (
                    Status {
                        temps,
                        channels,
                        ..Default::default()
                    },
                    nvidia_status.name.clone()
                )
            )
        }
        statuses
    }

    async fn get_nvidia_status(&self) -> Vec<StatusNvidia> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("nvidia-smi --query-gpu=index,gpu_name,temperature.gpu,utilization.gpu,fan.speed --format=csv,noheader,nounits")
            .output().await;
        match output {
            Ok(out) => {
                if out.status.success() {
                    let out_str = String::from_utf8(out.stdout).unwrap();
                    debug!("Nvidia raw status output: {}", out_str.trim());
                    let mut nvidia_statuses = vec![];
                    for line in out_str.trim().lines() {
                        if line.trim().is_empty() {
                            continue;  // skip any empty lines
                        }
                        let values = line.split(", ").collect::<Vec<&str>>();
                        if values.len() >= 5 {
                            let index = values[0].parse::<u8>();
                            if index.is_err() {
                                error!("Something is wrong with nvidia status output");
                                continue;
                            }
                            nvidia_statuses.push(StatusNvidia {
                                index: index.unwrap(),
                                name: values[1].to_string(),
                                temp: values[2].parse::<f64>().ok(),
                                load: values[3].parse::<u8>().ok(),
                                fan_duty: values[4].parse::<u8>().ok(),
                            });
                        }
                    }
                    return nvidia_statuses;
                } else {
                    let out_err = String::from_utf8(out.stderr).unwrap();
                    error!("Error communicating with nvidia-smi: {}", out_err)
                }
            }
            Err(err) => warn!("Nvidia driver not found: {}", err)
        }
        vec![]
    }
}

#[async_trait]
impl Repository for GpuRepo {
    async fn initialize_devices(&mut self) -> Result<()> {
        debug!("Starting Device Initialization");
        let start_initialization = Instant::now();
        self.detect_gpu_types().await;
        for (index, amd_device) in init_amd_devices().await.iter().enumerate() {
            let id = index as u8 + 1;
            let mut channels = HashMap::new();
            for channel in amd_device.channels.iter() {
                if channel.hwmon_type != HwmonChannelType::Fan {
                    continue;  // only Fan channels currently have controls
                }
                let channel_info = ChannelInfo {
                    speed_options: Some(SpeedOptions {
                        profiles_enabled: false,
                        fixed_enabled: true,
                        manual_profiles_enabled: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                channels.insert(channel.name.clone(), channel_info);
            }
            let mut status_channels = FanFns::extract_fan_statuses(amd_device).await;
            status_channels.extend(extract_load_status(amd_device).await);
            let status = Status {
                // todo: external names need to be adjusted "GPU#1 Temp1" for ex.
                channels: status_channels,
                temps: TempFns::extract_temp_statuses(&id, &amd_device).await,
                ..Default::default()
            };
            let mut device = Device {
                name: amd_device.name.clone(),
                d_type: DeviceType::GPU,
                type_id: id,
                info: Some(DeviceInfo {
                    channels,
                    temp_max: 100,
                    temp_ext_available: true,
                    model: amd_device.model.clone(),
                    ..Default::default()
                }),
                ..Default::default()
            };
            device.set_status(status);
            self.devices.insert(
                id,
                Arc::new(RwLock::new(device)),
            );
        }
        let has_multiple_gpus: bool = self.has_multiple_gpus.read().await.clone();
        let starting_nvidia_index = if has_multiple_gpus {
            self.gpu_type_count.read().await.get(&GpuType::AMD).unwrap_or(&0) + 1
        } else {
            1
        };
        for (index, (status, gpu_name)) in self.request_nvidia_statuses().await.into_iter().enumerate() {
            let id = index as u8 + starting_nvidia_index;
            // todo: also verify fan is writable...
            let mut device = Device {
                name: gpu_name,
                d_type: DeviceType::GPU,
                type_id: id,
                info: Some(DeviceInfo {
                    temp_max: 100,
                    temp_ext_available: true,
                    // channels:  // todo: Nvidia fan control channel if applicable
                    ..Default::default()
                }),
                ..Default::default()
            };
            device.set_status(status);
            self.devices.insert(
                id,
                Arc::new(RwLock::new(device)),
            );
        }
        let mut init_devices = vec![];
        for device in self.devices.values() {
            init_devices.push(device.read().await.clone())
        }
        debug!("Initialized Devices: {:?}", init_devices);
        debug!(
            "Time taken to initialize all GPU devices: {:?}", start_initialization.elapsed()
        );
        info!("All GPU devices initialized");
        Ok(())
    }

    async fn devices(&self) -> DeviceList {
        self.devices.values().cloned().collect()
    }

    async fn update_statuses(&self) -> Result<()> {
        debug!("Updating all GPU device statuses");
        let start_update = Instant::now();
        // todo: AMD
        for (index, (status, gpu_name)) in self.request_statuses().await.iter().enumerate() {
            let index = index as u8 + 1;
            if let Some(device_lock) = self.devices.get(&index) {
                device_lock.write().await.set_status(status.clone());
                debug!("Device: {} status updated: {:?}", gpu_name, status);
            }
        }
        debug!(
            "Time taken to update status for all GPU devices: {:?}",
            start_update.elapsed()
        );
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        debug!("GPU Repository shutdown");
        Ok(())
    }

    async fn apply_setting(&self, device_type_id: u8, setting: Setting) -> Result<()> {
        // todo: change nvidia fan
        //  nvidia-settings -a "[gpu:0]/GPUFanControlState=1" -a "[fan:0]/GPUTargetFanSpeed=25"
        // todo: amd? (is hwmon currently, but perhaps we move it in here (check the crates)
        todo!()
    }
}

async fn init_amd_devices() -> Vec<HwmonDriverInfo> {
    let base_paths = DeviceFns::find_all_hwmon_device_paths();
    let mut amd_devices = vec![];
    for path in base_paths {
        let device_name = DeviceFns::get_device_name(&path).await;
        if device_name != AMD_HWMON_NAME {
            continue;
        }
        let mut channels = vec![];
        match FanFns::init_fans(&path, &device_name).await {
            Ok(fans) => channels.extend(
                fans.into_iter().map(|fan| HwmonChannelInfo {
                    hwmon_type: fan.hwmon_type,
                    number: fan.number,
                    pwm_enable_default: fan.pwm_enable_default,
                    name: GPU_FAN_NAME.to_string(),
                    pwm_mode_supported: fan.pwm_mode_supported,
                }).collect::<Vec<HwmonChannelInfo>>()
            ),
            Err(err) => error!("Error initializing AMD Hwmon Fans: {}", err)
        };
        match TempFns::init_temps(&path, &device_name).await {
            Ok(temps) => channels.extend(
                temps.into_iter().map(|temp| HwmonChannelInfo {
                    hwmon_type: temp.hwmon_type,
                    number: temp.number,
                    name: GPU_TEMP_NAME.to_string(),
                    ..Default::default()
                }).collect::<Vec<HwmonChannelInfo>>()
            ),
            Err(err) => error!("Error initializing AMD Hwmon Temps: {}", err)
        };
        if let Some(load_channel) = init_amd_load(&path, &device_name).await {
            channels.push(load_channel)
        }
        let model = DeviceFns::get_device_model_name(&path).await;
        let hwmon_driver_info = HwmonDriverInfo {
            name: device_name,
            path,
            model,
            channels,
        };
        amd_devices.push(hwmon_driver_info);
    }
    amd_devices
}

async fn init_amd_load(base_path: &PathBuf, device_name: &String) -> Option<HwmonChannelInfo> {
    match tokio::fs::read_to_string(
        base_path.join("device").join("gpu_busy_percent")
    ).await {
        Ok(load) => match FanFns::check_parsing_8(load) {
            Ok(load_percent) => Some(HwmonChannelInfo {
                hwmon_type: HwmonChannelType::Load,
                name: GPU_LOAD_NAME.to_string(),
                ..Default::default()
            }),
            Err(err) => {
                warn!("Error reading AMD busy percent value: {}", err);
                None
            }
        }
        Err(_) => {
            warn!("No AMDGPU load found: {:?}/device/gpu_busy_percent", base_path);
            None
        }
    }
}

async fn extract_load_status(driver: &HwmonDriverInfo) -> Vec<ChannelStatus> {
    let mut channels = vec![];
    for channel in driver.channels.iter() {
        if channel.hwmon_type != HwmonChannelType::Load {
            continue;
        }
        let load = tokio::fs::read_to_string(
            driver.path.join("device").join("gpu_busy_percent")
        ).await
            .and_then(FanFns::check_parsing_8)
            .unwrap_or(0);
        channels.push(ChannelStatus {
            name: channel.name.clone(),
            rpm: None,
            duty: Some(load as f64),
            pwm_mode: None,
        })
    }
    channels
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusNvidia {
    index: u8,
    name: String,
    temp: Option<f64>,
    load: Option<u8>,
    fan_duty: Option<u8>,
}