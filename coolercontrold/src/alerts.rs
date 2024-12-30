/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2021-2024  Guy Boldon, Eren Simsek and contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::api::actor::AlertHandle;
use crate::api::CCError;
use crate::config::DEFAULT_CONFIG_DIR;
use crate::device::UID;
use crate::setting::{ChannelMetric, ChannelSource};
use crate::{cc_fs, AllDevices};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use const_format::concatcp;
use hashlink::LinkedHashMap;
use lazy_format::lazy_format;
use log::{error, info};
use moro_local::Scope;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Display;
use std::ops::Not;
use std::path::Path;
use std::rc::Rc;
use strum::{Display, EnumString};
use tokio_util::sync::CancellationToken;

const DEFAULT_ALERT_CONFIG_FILE_PATH: &str = concatcp!(DEFAULT_CONFIG_DIR, "/alerts.json");
const LOG_BUFFER_SIZE: usize = 20;

pub type AlertName = String;
pub type AlertLogMessage = String;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Alert {
    pub uid: UID,
    pub name: AlertName,
    pub channel_source: ChannelSource,
    pub min: f64,
    pub max: f64,
    pub state: AlertState,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString, Serialize, Deserialize, JsonSchema)]
pub enum AlertState {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AlertLog {
    pub uid: UID,
    pub name: AlertName,
    pub state: AlertState,
    pub message: AlertLogMessage,
    pub timestamp: DateTime<Local>,
}

impl Default for AlertLog {
    fn default() -> Self {
        AlertLog {
            uid: "Unknown".to_string(),
            name: "Unknown".to_string(),
            state: AlertState::Active,
            message: "Unknown".to_string(),
            timestamp: Local::now(),
        }
    }
}

pub struct AlertController {
    all_devices: AllDevices,
    alerts: RefCell<LinkedHashMap<UID, Alert>>,
    alert_handle: RefCell<Option<AlertHandle>>,
    logs: RefCell<VecDeque<AlertLog>>,
}

impl AlertController {
    /// A controller for managing and handling Alerts.
    pub async fn init<'s>(all_devices: AllDevices) -> Result<Self> {
        let alert_controller = Self {
            all_devices,
            alerts: RefCell::new(LinkedHashMap::new()),
            alert_handle: RefCell::new(None),
            logs: RefCell::new(VecDeque::with_capacity(LOG_BUFFER_SIZE)),
        };
        alert_controller.load_data_from_alert_config_file().await?;
        Ok(alert_controller)
    }

    /// Watches for shutdown and saves the current Alert data to the Alert configuration file.
    pub fn watch_for_shutdown<'s>(
        controller: &Rc<AlertController>,
        cancellation_token: CancellationToken,
        main_scope: &'s Scope<'s, 's, Result<()>>,
    ) {
        let alert_controller = controller.clone();
        main_scope.spawn(async move {
            cancellation_token.cancelled().await;
            info!("Shutting down Alert Controller");
            let _ = alert_controller.save_alert_data_to_config().await;
        });
    }

    /// Sets the `AlertHandle` for the `AlertController`.
    ///
    /// The `AlertHandle` is used to broadcast notifications when an `Alert` state changes.
    pub fn set_alert_handle(&self, alert_handle: AlertHandle) {
        self.alert_handle.replace(Some(alert_handle));
    }

    /// Reads the Alert configuration file and fills the Alert `HashMap`.
    async fn load_data_from_alert_config_file(&self) -> Result<()> {
        let config_dir = Path::new(DEFAULT_CONFIG_DIR);
        if !config_dir.exists() {
            info!(
                "config directory doesn't exist. Attempting to create it: {}",
                DEFAULT_CONFIG_DIR
            );
            cc_fs::create_dir_all(config_dir)?;
        }
        let path = Path::new(DEFAULT_ALERT_CONFIG_FILE_PATH).to_path_buf();
        let config_contents = if let Ok(contents) = cc_fs::read_txt(&path).await {
            contents
        } else {
            info!("Writing a new Alerts configuration file");
            let default_alert_config = serde_json::to_string(&AlertConfigFile {
                alerts: Vec::with_capacity(0),
                logs: Vec::with_capacity(0),
            })?;
            cc_fs::write_string(&path, default_alert_config)
                .await
                .with_context(|| format!("Writing new configuration file: {path:?}"))?;
            // make sure the file is readable:
            cc_fs::read_txt(&path)
                .await
                .with_context(|| format!("Reading configuration file {path:?}"))?
        };
        let alert_config: AlertConfigFile = serde_json::from_str(&config_contents)
            .with_context(|| format!("Parsing Alert configuration file {path:?}"))?;
        {
            let mut alerts_lock = self.alerts.borrow_mut();
            alerts_lock.clear();
            for alert in alert_config.alerts {
                alerts_lock.insert(alert.uid.clone(), alert);
            }
        }
        {
            let mut logs_lock = self.logs.borrow_mut();
            logs_lock.clear();
            logs_lock.extend(alert_config.logs);
        }
        Ok(())
    }

    /// Saves the current Alert data to the Alert configuration file.
    async fn save_alert_data_to_config(&self) -> Result<()> {
        let alert_config = AlertConfigFile {
            alerts: self.alerts.borrow().values().cloned().collect(),
            logs: self.logs.borrow().iter().cloned().collect(),
        };
        let alert_config_json = serde_json::to_string(&alert_config)?;
        cc_fs::write_string(DEFAULT_ALERT_CONFIG_FILE_PATH, alert_config_json)
            .await
            .with_context(|| "Writing Alert Configuration File")?;
        Ok(())
    }

    /// Returns a tuple of all available Alerts and logs: (alerts, logs)
    pub fn get_all(&self) -> (Vec<Alert>, Vec<AlertLog>) {
        let alerts = self.alerts.borrow().values().cloned().collect();
        let logs = self.logs.borrow().iter().cloned().collect();
        (alerts, logs)
    }

    /// Creates a new Alert
    pub async fn create(&self, alert: Alert) -> Result<()> {
        if self.alerts.borrow().contains_key(&alert.uid) {
            return Err(CCError::UserError {
                msg: format!("Alert with uid {} already exists", alert.uid),
            }
            .into());
        }
        self.alerts.borrow_mut().insert(alert.uid.clone(), alert);
        self.save_alert_data_to_config().await
    }

    /// Updates an existing Alert
    pub async fn update(&self, mut alert: Alert) -> Result<()> {
        {
            let mut alerts_lock = self.alerts.borrow_mut();
            if alerts_lock.contains_key(&alert.uid).not() {
                return Err(CCError::NotFound {
                    msg: format!("Alert with uid {} does not exist", alert.uid),
                }
                .into());
            }
            // don't overwrite state:
            let current_state = alerts_lock.get(&alert.uid).unwrap().state.clone();
            alert.state = current_state;
            alerts_lock.insert(alert.uid.clone(), alert);
        }
        self.save_alert_data_to_config().await
    }

    /// Deletes an existing Alert
    pub async fn delete(&self, alert_uid: UID) -> Result<()> {
        if self.alerts.borrow().contains_key(&alert_uid).not() {
            return Err(CCError::NotFound {
                msg: format!("Alert with uid {alert_uid} does not exist"),
            }
            .into());
        }
        self.alerts.borrow_mut().remove(&alert_uid);
        self.save_alert_data_to_config().await
    }

    /// Processes all Alerts, firing off messages if an alert state has changed.
    /// This function should be called in the main loop
    pub fn process_alerts(&self) {
        let alerts_to_fire = self.process_and_collect_alerts_to_fire();
        for alert_data in alerts_to_fire {
            let log = self.log_alert_state_change(
                alert_data.0.uid,
                alert_data.0.name,
                alert_data.0.state,
                alert_data.1,
            );
            if let Some(handle) = self.alert_handle.borrow().as_ref() {
                handle.broadcast_alert_state_change(log);
            }
        }
    }

    /// Collects all Alerts that need firing
    #[allow(clippy::too_many_lines)]
    fn process_and_collect_alerts_to_fire(&self) -> Vec<(Alert, AlertLogMessage)> {
        let mut alerts_to_fire = Vec::new();
        for alert in self.alerts.borrow_mut().values_mut() {
            let Some(device) = self.all_devices.get(&alert.channel_source.device_uid) else {
                Self::activate_alert(&mut alerts_to_fire, alert, "Device not found");
                continue;
            };
            let most_recent_status = device.borrow().status_current().unwrap();
            let channel_value = if alert.channel_source.channel_metric == ChannelMetric::Temp {
                let Some(temp_status) = most_recent_status
                    .temps
                    .iter()
                    .find(|temp| temp.name == alert.channel_source.channel_name)
                else {
                    Self::activate_alert(&mut alerts_to_fire, alert, "Device Channel not found");
                    continue;
                };
                temp_status.temp
            } else {
                let Some(channel_status) = most_recent_status
                    .channels
                    .iter()
                    .find(|channel| channel.name == alert.channel_source.channel_name)
                else {
                    Self::activate_alert(&mut alerts_to_fire, alert, "Device Channel not found");
                    continue;
                };
                match alert.channel_source.channel_metric {
                    ChannelMetric::Duty => {
                        let Some(duty) = channel_status.duty else {
                            Self::activate_alert(
                                &mut alerts_to_fire,
                                alert,
                                "Device Channel Duty Metric not found",
                            );
                            continue;
                        };
                        duty
                    }
                    ChannelMetric::Load => {
                        let Some(load) = channel_status.duty else {
                            Self::activate_alert(
                                &mut alerts_to_fire,
                                alert,
                                "Device Channel Load Metric not found",
                            );
                            continue;
                        };
                        load
                    }
                    ChannelMetric::RPM => {
                        let Some(rpm) = channel_status.rpm else {
                            Self::activate_alert(
                                &mut alerts_to_fire,
                                alert,
                                "Device Channel RPM Metric not found",
                            );
                            continue;
                        };
                        f64::from(rpm)
                    }
                    ChannelMetric::Freq => {
                        let Some(freq) = channel_status.freq else {
                            Self::activate_alert(
                                &mut alerts_to_fire,
                                alert,
                                "Device Channel Freq Metric not found",
                            );
                            continue;
                        };
                        f64::from(freq)
                    }
                    ChannelMetric::Temp => {
                        error!("This should not happen, ChannelMetric::TEMP should already be handled.");
                        continue;
                    }
                }
            };
            if channel_value > alert.max {
                if alert.state == AlertState::Active {
                    continue;
                }
                let channel_name = alert.channel_source.channel_name.clone();
                let max = alert.max;
                Self::activate_alert(
                    &mut alerts_to_fire,
                    alert,
                    lazy_format!(
                        "{channel_name}: {channel_value} is greater than allowed maximum: {max}"
                    ),
                );
            } else if channel_value < alert.min {
                if alert.state == AlertState::Active {
                    continue;
                }
                let channel_name = alert.channel_source.channel_name.clone();
                let min = alert.min;
                Self::activate_alert(
                    &mut alerts_to_fire,
                    alert,
                    lazy_format!(
                        "{channel_name}: {channel_value} is less than allowed minimum: {min}"
                    ),
                );
            } else if alert.state != AlertState::Inactive {
                let channel_name = alert.channel_source.channel_name.clone();
                let min = alert.min;
                let max = alert.max;
                Self::deactivate_alert(
                    &mut alerts_to_fire,
                    alert,
                    format!(
                    "{channel_name}: {channel_value} is again within allowed range: {min} - {max}"
                ),
                );
            }
        }
        alerts_to_fire
    }

    /// Adds an Alert to the list of alerts to fire, if state has changed.
    fn activate_alert(
        alerts_to_fire: &mut Vec<(Alert, AlertLogMessage)>,
        alert: &mut Alert,
        message: impl Display,
    ) {
        if alert.state == AlertState::Active {
            return; // only fire on state change
        }
        alert.state = AlertState::Active;
        alerts_to_fire.push((alert.clone(), message.to_string()));
    }

    /// Adds an Alert to the list of alerts to fire. State change should be checked before calling
    /// this method.
    fn deactivate_alert(
        alerts_to_fire: &mut Vec<(Alert, AlertLogMessage)>,
        alert: &mut Alert,
        message: String,
    ) {
        alert.state = AlertState::Inactive;
        alerts_to_fire.push((alert.clone(), message));
    }

    /// Logs an alert state change to the internal buffer, as well as returning the newly
    /// created log entry.
    pub fn log_alert_state_change(
        &self,
        uid: UID,
        name: AlertName,
        state: AlertState,
        message: AlertLogMessage,
    ) -> AlertLog {
        let log = AlertLog {
            uid,
            name,
            state,
            message,
            timestamp: Local::now(),
        };
        let mut logs_lock = self.logs.borrow_mut();
        if logs_lock.len() > LOG_BUFFER_SIZE {
            logs_lock.pop_front();
        }
        logs_lock.push_back(log.clone());
        log
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertConfigFile {
    alerts: Vec<Alert>,
    logs: Vec<AlertLog>,
}
