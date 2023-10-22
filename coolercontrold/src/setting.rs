/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2023  Guy Boldon
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
 */


use std::time::Duration;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::device::UID;

/// Setting is a passed struct used to apply various settings to a specific device.
/// Usually only one specific lighting or speed setting is applied at a time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub channel_name: String,

    /// The fixed duty speed to set. eg: 20 (%)
    pub speed_fixed: Option<u8>,

    /// The profile temp/duty speeds to set. eg: [(20.0, 50), (25.7, 80)]
    pub speed_profile: Option<Vec<(f64, u8)>>,

    /// The associated temperature source
    pub temp_source: Option<TempSource>,

    /// Settings for lighting
    pub lighting: Option<LightingSettings>,

    /// Settings for LCD screens
    pub lcd: Option<LcdSettings>,

    /// the current pwm_mode to set for hwmon devices, eg: 1
    pub pwm_mode: Option<u8>,

    /// Used to set hwmon & nvidia channels back to their default 'automatic' values.
    pub reset_to_default: Option<bool>,

}

impl Default for Setting {
    fn default() -> Self {
        Self {
            channel_name: "".to_string(),
            speed_fixed: None,
            speed_profile: None,
            temp_source: None,
            lighting: None,
            lcd: None,
            pwm_mode: None,
            reset_to_default: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingSettings {
    /// The lighting mode name
    pub mode: String,

    /// The speed to set
    pub speed: Option<String>,

    /// run backwards or not
    pub backward: Option<bool>,

    /// a list of RGB tuple values, eg [(20,20,120), (0,0,255)]
    pub colors: Vec<(u8, u8, u8)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempSource {
    /// The internal name for this Temperature Source. Not the frontend_name or external_name
    pub temp_name: String,

    /// The associated device uid containing current temp values
    pub device_uid: UID,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcdSettings {
    /// The Lcd mode name
    pub mode: String,

    /// The LCD brightness (0-100%)
    pub brightness: Option<u8>,

    /// The LCD Image orientation (0,90,180,270)
    pub orientation: Option<u16>,

    /// The LCD Source Image file path location
    pub image_file_src: Option<String>,

    /// The LCD Image tmp file path location, where the preprocessed image is located
    pub image_file_processed: Option<String>,

    /// a list of RGB tuple values, eg [(20,20,120), (0,0,255)]
    pub colors: Vec<(u8, u8, u8)>,
}

/// General Settings for CoolerControl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoolerControlSettings {
    pub apply_on_boot: bool,
    pub no_init: bool,
    pub handle_dynamic_temps: bool,
    pub startup_delay: Duration,
    pub smoothing_level: u8,
    pub thinkpad_full_speed: bool,
}

/// General Device Settings for CoolerControl
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoolerControlDeviceSettings {
    pub disable: bool,
}

/// Profile Settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// The Unique Identifier for this Profile
    pub uid: UID,

    /// The profile type
    pub p_type: ProfileType,

    /// The User given name for this Profile
    pub name: String,

    /// The fixed duty speed to set. eg: 20 (%)
    pub speed_fixed: Option<u8>,

    /// The profile temp/duty speeds to set. eg: [(20.0, 50), (25.7, 80)]
    pub speed_profile: Option<Vec<(f64, u8)>>,

    /// The associated temperature source
    pub temp_source: Option<TempSource>,

    /// The function uid to apply to this profile
    pub function_uid: UID,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            uid: "0".to_string(),
            p_type: ProfileType::Default,
            name: "Default Profile".to_string(),
            speed_fixed: None,
            speed_profile: None,
            temp_source: None,
            function_uid: "0".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
pub enum ProfileType {
    Default,
    Fixed,
    Graph,
    Mix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// The Unique identifier for this function
    pub uid: UID,

    /// The user given name for this function
    pub name: String,

    /// The type of this function
    pub f_type: FunctionType,

    /// The response delay in seconds
    pub response_delay: Option<u8>,

    /// The temperature deviance threshold in degrees
    pub deviance: Option<f64>,

    /// The sample window this function should use, particularly applicable to moving averages
    pub sample_window: Option<u16>,
}

impl Default for Function {
    fn default() -> Self {
        Self {
            uid: "0".to_string(),
            name: "Identity".to_string(),
            f_type: FunctionType::Identity,
            response_delay: None,
            deviance: None,
            sample_window: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
pub enum FunctionType {
    Identity,
    Standard,
    SimpleMovingAvg,
    ExponentialMovingAvg,
}
