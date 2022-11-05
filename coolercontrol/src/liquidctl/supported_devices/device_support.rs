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
use std::fmt::Debug;

use heck::ToTitleCase;
use lazy_static::lazy_static;
use regex::Regex;

use crate::device::{ChannelStatus, DeviceInfo, LightingMode, Status, TempStatus};

type StatusMap = HashMap<String, String>;

fn parse_float(value: &String) -> Option<f64> {
    value.parse::<f64>().ok()
}

fn parse_u32(value: &String) -> Option<u32> {
    value.parse::<u32>().ok()
}

/// It is a general purpose trait and each supported device struc must implement this trait.
/// Many of the default methods will cover all use cases and it is advisable to override them
/// for increase efficiency and performance.
pub trait DeviceSupport: Debug + Sync + Send {
    // todo: device_instance won't be available for this method,
    //  we may need to send extra data from liqctld for this
    fn extract_info(&self) -> DeviceInfo;

    // todo: in python this is an *args: Any parameter... that won't fly here.
    fn get_filtered_color_channel_modes(&self) -> Vec<LightingMode>;

    fn extract_status(&self, status_map: &StatusMap, device_id: &u8) -> Status {
        Status {
            firmware_version: self.get_firmware_ver(status_map),
            temps: self.get_temperatures(status_map, device_id),
            channels: self.get_channel_statuses(status_map, device_id),
            ..Default::default()
        }
    }

    fn get_firmware_ver(&self, status_map: &StatusMap) -> Option<String> {
        status_map.get("firmware version").cloned()
    }


    /// It's possible to override this method and use only the needed sub-functions per device
    fn get_temperatures(&self,
                        status_map: &StatusMap,
                        device_id: &u8,
    ) -> Vec<TempStatus> {
        let mut temps = vec![];
        self.add_liquid_temp(status_map, &mut temps, device_id);
        self.add_water_temp(status_map, &mut temps, device_id);
        self.add_temp(status_map, &mut temps, device_id);
        self.add_temp_probes(status_map, &mut temps, device_id);
        self.add_noise_level(status_map, &mut temps, device_id);
        temps
    }

    fn add_liquid_temp(&self, status_map: &StatusMap, temps: &mut Vec<TempStatus>, device_id: &u8) {
        let liquid_temp = status_map.get("liquid temperature")
            .and_then(parse_float);
        if let Some(temp) = liquid_temp {
            temps.push(TempStatus {
                name: "liquid".to_string(),
                temp,
                frontend_name: "Liquid".to_string(),
                external_name: format!("LC#{} Liquid", device_id),
            })
        }
    }

    fn add_water_temp(&self, status_map: &StatusMap, temps: &mut Vec<TempStatus>, device_id: &u8) {
        let water_temp = status_map.get("water temperature")
            .and_then(parse_float);
        if let Some(temp) = water_temp {
            temps.push(TempStatus {
                name: "water".to_string(),
                temp,
                frontend_name: "Water".to_string(),
                external_name: format!("LC#{} Water", device_id),
            })
        }
    }

    fn add_temp(&self, status_map: &StatusMap, temps: &mut Vec<TempStatus>, device_id: &u8) {
        let plain_temp = status_map.get("temperature")
            .and_then(parse_float);
        if let Some(temp) = plain_temp {
            temps.push(TempStatus {
                name: "temp".to_string(),
                temp,
                frontend_name: "Temp".to_string(),
                external_name: format!("LC#{} Temp", device_id),
            })
        }
    }

    fn add_temp_probes(&self, status_map: &StatusMap, temps: &mut Vec<TempStatus>, device_id: &u8) {
        lazy_static!(
            static ref TEMP_PROB_PATTERN: Regex = Regex::new(r"temperature \d+").unwrap();
            static ref NUMBER_PATTERN: Regex = Regex::new(r"\d+").unwrap();
        );
        for (probe_name, value) in status_map.iter() {
            if TEMP_PROB_PATTERN.is_match(probe_name) {
                if let Some(temp) = parse_float(value) {
                    if let Some(probe_number) = NUMBER_PATTERN.find_at(probe_name, probe_name.len() - 2) {
                        let name = format!("temp{}", probe_number.as_str());
                        temps.push(TempStatus {
                            temp,
                            frontend_name: name.to_title_case(),
                            external_name: format!("LC#{} {}", device_id, name.to_title_case()),
                            name,
                        })
                    }
                }
            }
        }
    }

    fn add_noise_level(&self, status_map: &StatusMap, temps: &mut Vec<TempStatus>, device_id: &u8) {
        let noise_lvl = status_map.get("noise level")
            .and_then(parse_float);
        if let Some(noise) = noise_lvl {
            temps.push(TempStatus {
                name: "noise".to_string(),
                temp: noise,
                frontend_name: "Noise dB".to_string(),
                external_name: format!("LC#{} Noise dB", device_id),
            })
        }
    }

    /// It's possible to override this method and use only the needed sub-functions per device
    fn get_channel_statuses(&self, status_map: &StatusMap, device_id: &u8) -> Vec<ChannelStatus> {
        let mut channel_statuses = vec![];
        self.add_single_fan_status(status_map, &mut channel_statuses);
        self.add_single_pump_status(status_map, &mut channel_statuses);
        self.add_multiple_fans_status(status_map, &mut channel_statuses);
        channel_statuses
    }

    fn add_single_fan_status(&self, status_map: &StatusMap, channel_statuses: &mut Vec<ChannelStatus>) {
        let fan_rpm = status_map.get("fan speed")
            .and_then(parse_u32);
        let fan_duty = status_map.get("fan duty")
            .and_then(parse_float);
        if fan_rpm.is_some() || fan_duty.is_some() {
            channel_statuses.push(
                ChannelStatus {
                    name: "fan".to_string(),
                    rpm: fan_rpm,
                    duty: fan_duty,
                    pwm_mode: None,
                }
            )
        }
    }

    fn add_single_pump_status(&self, status_map: &StatusMap, channel_statuses: &mut Vec<ChannelStatus>) {
        let pump_rpm = status_map.get("pump speed")
            .and_then(parse_u32);
        let pump_duty = status_map.get("pump duty")
            .and_then(parse_float);
        if pump_rpm.is_some() || pump_duty.is_some() {
            channel_statuses.push(
                ChannelStatus {
                    name: "pump".to_string(),
                    rpm: pump_rpm,
                    duty: pump_duty,
                    pwm_mode: None,
                }
            )
        }
    }

    /// This is used for special devices with limited pump speeds that are named (str)
    fn get_pump_mode(&self, status_map: &StatusMap) -> Option<String> {
        status_map.get("pump mode").cloned()
    }

    fn add_multiple_fans_status(&self,
                                status_map: &StatusMap,
                                channel_statuses: &mut Vec<ChannelStatus>,
    ) {
        lazy_static!(
            static ref NUMBER_PATTERN: Regex = Regex::new(r"\d+").unwrap();
            static ref MULTIPLE_FAN_SPEED: Regex = Regex::new(r"fan \d+ speed").unwrap();
            static ref MULTIPLE_FAN_SPEED_CORSAIR: Regex = Regex::new(r"fan speed \d+").unwrap();
            static ref MULTIPLE_FAN_DUTY: Regex = Regex::new(r"fan \d+ duty").unwrap();
        );
        let mut fans_map: HashMap<String, (Option<u32>, Option<f64>)> = HashMap::new();
        for (name, value) in status_map.iter() {
            if let Some(fan_number) = NUMBER_PATTERN.find_at(name, 3)
                .and_then(|number| parse_u32(&number.as_str().to_string())) {
                let fan_name = format!("fan{}", fan_number);
                if MULTIPLE_FAN_SPEED.is_match(name) || MULTIPLE_FAN_SPEED_CORSAIR.is_match(name) {
                    let (rpm, _) = fans_map
                        .entry(fan_name)
                        .or_insert((None, None));
                    *rpm = parse_u32(value);
                } else if MULTIPLE_FAN_DUTY.is_match(name) {
                    let (_, duty) = fans_map
                        .entry(fan_name)
                        .or_insert((None, None));
                    *duty = parse_float(value);
                }
            }
        }
        for (name, (rpm, duty)) in fans_map {
            channel_statuses.push(
                ChannelStatus { name, rpm, duty, pwm_mode: None }
            )
        }
    }

    fn channel_to_frontend_name(&self, lighting_channel: &String) -> String {
        lighting_channel.replace("-", " ").replace("_", " ").to_title_case()
    }
}

/// Support for the Liquidctl KrakenX3 Driver
#[derive(Debug)]
pub struct KrakenX3Support;

impl KrakenX3Support {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl DeviceSupport for KrakenX3Support {
    fn extract_info(&self) -> DeviceInfo {
        todo!()
    }

    fn get_filtered_color_channel_modes(&self) -> Vec<LightingMode> {
        todo!()
    }
}

/// Support for the Liquidctl SmartDevice2 Driver
#[derive(Debug)]
pub struct SmartDevice2Support;

impl SmartDevice2Support {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl DeviceSupport for SmartDevice2Support {
    fn extract_info(&self) -> DeviceInfo {
        todo!()
    }

    fn get_filtered_color_channel_modes(&self) -> Vec<LightingMode> {
        todo!()
    }
}

/// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_temp_status_vector_contents_eq(device_support: KrakenX3Support, device_id: &u8, given_expected: Vec<(HashMap<String, String>, Vec<TempStatus>)>) {
        for (given, expected) in given_expected {
            let result = device_support.get_temperatures(&given, &device_id);
            assert!(
                expected.iter().all(|temp_status| result.contains(&temp_status))
            );
            assert!(
                result.iter().all(|temp_status| expected.contains(&temp_status))
            );
        }
    }

    /// Using KrakenX3Support to test the Trait functions
    #[test]
    fn get_firmware() {
        let device_support = KrakenX3Support::new();
        let given_expected = vec![
            (
                HashMap::from([("firmware version".to_string(), "1.0.0".to_string())]),
                Some("1.0.0".to_string())
            ),
            (HashMap::from([("firmware".to_string(), "1.0.0".to_string())]), None),
            (
                HashMap::from([("firmware version".to_string(), "whatever".to_string())]),
                Some("whatever".to_string())
            ),
        ];
        for (given, expected) in given_expected {
            assert_eq!(
                device_support.get_firmware_ver(&given),
                expected
            )
        }
    }

    #[test]
    fn get_temperatures_fail() {
        let device_support = KrakenX3Support::new();
        let temp = "33.3".to_string();
        let device_id: u8 = 1;
        let given_expected = vec![
            (
                HashMap::from([("liquid temperature".to_string(), "whatever".to_string())]),
                vec![]
            ),
            (HashMap::from([("some other temperature".to_string(), temp.clone())]), vec![]),
        ];
        for (given, expected) in given_expected {
            let result = device_support.get_temperatures(&given, &device_id);
            assert!(
                expected.iter().all(|temp_status| !result.contains(&temp_status))
            );
            assert!(
                result.iter().all(|temp_status| !expected.contains(&temp_status))
            );
        }
    }

    #[test]
    fn add_liquid_temp() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let temp = "33.3".to_string();
        let given_expected = vec![
            (
                HashMap::from([("liquid temperature".to_string(), temp.clone())]),
                vec![TempStatus {
                    name: "liquid".to_string(),
                    temp: temp.parse().unwrap(),
                    frontend_name: "Liquid".to_string(),
                    external_name: "LC#1 Liquid".to_string(),
                }]
            ),
        ];
        assert_temp_status_vector_contents_eq(device_support, &device_id, given_expected)
    }


    #[test]
    fn add_water_temp() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let temp = "33.3".to_string();
        let given_expected = vec![
            (
                HashMap::from([("water temperature".to_string(), temp.clone())]),
                vec![TempStatus {
                    name: "water".to_string(),
                    temp: temp.parse().unwrap(),
                    frontend_name: "Water".to_string(),
                    external_name: "LC#1 Water".to_string(),
                }]
            ),
        ];
        assert_temp_status_vector_contents_eq(device_support, &device_id, given_expected)
    }

    #[test]
    fn add_plain_temp() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let temp = "33.3".to_string();
        let given_expected = vec![
            (
                HashMap::from([("temperature".to_string(), temp.clone())]),
                vec![TempStatus {
                    name: "temp".to_string(),
                    temp: temp.parse().unwrap(),
                    frontend_name: "Temp".to_string(),
                    external_name: "LC#1 Temp".to_string(),
                }]
            ),
        ];
        assert_temp_status_vector_contents_eq(device_support, &device_id, given_expected)
    }


    #[test]
    fn add_temp_probes() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let temp = "33.3".to_string();
        let given_expected = vec![
            (
                HashMap::from([
                    ("temperature 1".to_string(), temp.clone()),
                    ("temperature 2".to_string(), temp.clone()),
                    ("temperature 3".to_string(), temp.clone()),
                ]),
                vec![
                    TempStatus {
                        name: "temp1".to_string(),
                        temp: temp.parse().unwrap(),
                        frontend_name: "Temp1".to_string(),
                        external_name: "LC#1 Temp1".to_string(),
                    },
                    TempStatus {
                        name: "temp2".to_string(),
                        temp: temp.parse().unwrap(),
                        frontend_name: "Temp2".to_string(),
                        external_name: "LC#1 Temp2".to_string(),
                    },
                    TempStatus {
                        name: "temp3".to_string(),
                        temp: temp.parse().unwrap(),
                        frontend_name: "Temp3".to_string(),
                        external_name: "LC#1 Temp3".to_string(),
                    },
                ]
            ),
        ];
        assert_temp_status_vector_contents_eq(device_support, &device_id, given_expected)
    }

    #[test]
    fn add_noise_level() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let noise_lvl = "33.3".to_string();
        let given_expected = vec![
            (
                HashMap::from([("noise level".to_string(), noise_lvl.clone())]),
                vec![TempStatus {
                    name: "noise".to_string(),
                    temp: noise_lvl.parse().unwrap(),
                    frontend_name: "Noise dB".to_string(),
                    external_name: "LC#1 Noise dB".to_string(),
                }]
            ),
        ];
        assert_temp_status_vector_contents_eq(device_support, &device_id, given_expected)
    }

    fn assert_channel_statuses_eq(device_support: KrakenX3Support, device_id: &u8, given_expected: Vec<(HashMap<String, String>, Vec<ChannelStatus>)>) {
        for (given, expected) in given_expected {
            let result = device_support.get_channel_statuses(&given, &device_id);
            assert!(
                expected.iter().all(|temp_status| result.contains(&temp_status))
            );
            assert!(
                result.iter().all(|temp_status| expected.contains(&temp_status))
            );
        }
    }

    #[test]
    fn add_single_fan_status() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let rpm: u32 = 33;
        let duty: f64 = 33.3;
        let given_expected = vec![
            (
                HashMap::from([
                    ("fan speed".to_string(), rpm.to_string()),
                    ("fan duty".to_string(), duty.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "fan".to_string(),
                    rpm: Some(rpm),
                    duty: Some(duty),
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn add_single_fan_status_rpm() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let rpm: u32 = 33;
        let given_expected = vec![
            (
                HashMap::from([
                    ("fan speed".to_string(), rpm.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "fan".to_string(),
                    rpm: Some(rpm),
                    duty: None,
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn add_single_fan_status_duty() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let duty: f64 = 33.3;
        let given_expected = vec![
            (
                HashMap::from([
                    ("fan duty".to_string(), duty.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "fan".to_string(),
                    rpm: None,
                    duty: Some(duty),
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn add_single_pump_status() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let rpm: u32 = 33;
        let duty: f64 = 33.3;
        let given_expected = vec![
            (
                HashMap::from([
                    ("pump speed".to_string(), rpm.to_string()),
                    ("pump duty".to_string(), duty.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "pump".to_string(),
                    rpm: Some(rpm),
                    duty: Some(duty),
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn add_single_pump_status_rpm() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let rpm: u32 = 33;
        let given_expected = vec![
            (
                HashMap::from([
                    ("pump speed".to_string(), rpm.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "pump".to_string(),
                    rpm: Some(rpm),
                    duty: None,
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn add_single_pump_status_duty() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let duty: f64 = 33.3;
        let given_expected = vec![
            (
                HashMap::from([
                    ("pump duty".to_string(), duty.to_string()),
                ]),
                vec![ChannelStatus {
                    name: "pump".to_string(),
                    rpm: None,
                    duty: Some(duty),
                    pwm_mode: None,
                }]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }

    #[test]
    fn get_pump_mode() {
        let device_support = KrakenX3Support::new();
        let status_map = HashMap::from([("pump mode".to_string(), "balanced".to_string())]);
        let result = device_support.get_pump_mode(&status_map);
        assert_eq!(result, Some("balanced".to_string()));
    }

    #[test]
    fn channel_to_frontend_name() {
        let device_support = KrakenX3Support::new();
        let channel_name = "here_is-the_channel-name".to_string();
        let result = device_support.channel_to_frontend_name(&channel_name);
        assert_eq!(result, "Here Is The Channel Name".to_string());
    }


    #[test]
    fn add_multiple_fans() {
        let device_support = KrakenX3Support::new();
        let device_id: u8 = 1;
        let rpm: u32 = 33;
        let duty: f64 = 33.3;
        let given_expected = vec![
            (
                HashMap::from([
                    ("fan 1 speed".to_string(), rpm.to_string()),
                    ("fan 1 duty".to_string(), duty.to_string()),
                    ("fan 2 speed".to_string(), rpm.to_string()),
                    ("fan 3 duty".to_string(), duty.to_string()),
                    ("fan speed 4".to_string(), rpm.to_string()),
                ]),
                vec![
                    ChannelStatus {
                        name: "fan1".to_string(),
                        rpm: Some(rpm),
                        duty: Some(duty),
                        pwm_mode: None,
                    },
                    ChannelStatus {
                        name: "fan2".to_string(),
                        rpm: Some(rpm),
                        duty: None,
                        pwm_mode: None,
                    },
                    ChannelStatus {
                        name: "fan3".to_string(),
                        rpm: None,
                        duty: Some(duty),
                        pwm_mode: None,
                    },
                    ChannelStatus {
                        name: "fan4".to_string(),
                        rpm: Some(rpm),
                        duty: None,
                        pwm_mode: None,
                    },
                ]
            ),
        ];
        assert_channel_statuses_eq(device_support, &device_id, given_expected);
    }
}