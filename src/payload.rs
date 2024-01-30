use crate::consts::*;
use crate::payload;
use pzem016lib::PZEM;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use pzem016lib::errors::PZEMError;
use tokio::sync::mpsc;
use tokio::time::sleep;
use crate::ipc::{IPCMessage, PublishMessage};
use crate::SHUTDOWN;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeviceInfo {
    pub identifiers: Vec<String>,
    pub manufacturer: String,
    pub name: String,
    pub model: String,
    pub sw_version: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(untagged)]
pub enum PayloadValueType {
    Float(f32),
    Int(i64),
    String(String),
    Boolean(bool),
    #[default]
    None,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Config(HAConfigPayload),
    CurrentState(StatePayload),
    #[default]
    None,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityCategory {
    Config,
    #[default]
    Diagnostic,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HAConfigPayload {
    pub name: String,
    pub device: DeviceInfo,
    pub unique_id: String,
    pub entity_id: String,
    pub state_topic: String,
    pub expires_after: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_category: Option<EntityCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_off: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "unit_of_measurement")]
    pub native_uom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_display_precision: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumed_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_state_attributes: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_entity_name: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_poll: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_press: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePayload {
    pub value: PayloadValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub last_seen: SystemTime,
}

impl Default for StatePayload {
    fn default() -> Self {
        StatePayload {
            value: PayloadValueType::None,
            last_seen: SystemTime::now(),
            description: None,
            label: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompoundPayload {
    pub(crate) config: HAConfigPayload,
    pub(crate) config_topic: String,
    pub(crate) state: StatePayload,
    pub(crate) state_topic: String,
}

pub async fn generate_payloads(pzem: &mut PZEM, tx: tokio::sync::mpsc::Sender<IPCMessage>) -> Result<(),PZEMError>{
    loop {
        if let Some(shutting_down) = SHUTDOWN.get() {
                    return Err(PZEMError::ExitingThread);
                }
        for unit_id in 101..=140 {
            let model: String = format!("pzem016");
            let serial: String = format!("{unit_id}");
            let device_info = payload::DeviceInfo {
                identifiers: vec![serial.clone()],
                manufacturer: "Peacefair".to_string(),
                name: "PZEM-016".to_string(),
                model: model.clone(),
                sw_version: "".to_string(),
            };
            let unit_name = format!("{model}-{serial}");
            let mut config_payload: HAConfigPayload = HAConfigPayload::default();
            let mut state_payload: StatePayload = StatePayload::default();

            let data = pzem.get_data(unit_id).await.expect("couldn't read data");

            //volts
            let config_topic: String = format!("homeassistant/sensor/pzem016-{serial}/volts/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/volts/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-voltage", unit_name.clone());
            config_payload.device_class = Some("voltage".to_string());
            config_payload.state_class = Some("measurement".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(1);
            config_payload.native_uom = Some("V".to_string());
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.volts as f32);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }

            //current
            let config_topic: String = format!("homeassistant/sensor/pzem016-{serial}/current/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/current/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-current", unit_name.clone());
            config_payload.device_class = Some("current".to_string());
            config_payload.state_class = Some("measurement".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(1);
            config_payload.native_uom = Some("A".to_string());
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.amps as f32);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }

            //power
            let config_topic: String = format!("homeassistant/sensor/pzem016-{serial}/power/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/power/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-power", unit_name.clone());
            config_payload.device_class = Some("power".to_string());
            config_payload.state_class = Some("measurement".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(1);
            config_payload.native_uom = Some("W".to_string());
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.watts as f32);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }

            //energy
            let config_topic: String = format!("homeassistant/sensor/pzem016-{serial}/energy/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/energy/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-energy", unit_name.clone());
            config_payload.device_class = Some("energy".to_string());
            config_payload.state_class = Some("total_increasing".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(1);
            config_payload.native_uom = Some("Wh".to_string());
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.watt_hours as f32);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }

            // frequency
            let config_topic: String =
                format!("homeassistant/sensor/pzem016-{serial}/frequency/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/frequency/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-frequency", unit_name.clone());
            config_payload.device_class = Some("frequency".to_string());
            config_payload.state_class = Some("measurement".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(1);
            config_payload.native_uom = Some("Hz".to_string());
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.frequency as f32);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }

            // power_factor
            let config_topic: String =
                format!("homeassistant/sensor/pzem016-{serial}/power_factor/config");
            let state_topic = format!("pzem016mqtt/pzem016-{serial}/power_factor/value");
            config_payload.state_topic = state_topic.clone();
            config_payload.name = format!("{}-power_factor", unit_name.clone());
            config_payload.device_class = Some("power_factor".to_string());
            config_payload.state_class = Some("measurement".to_string());
            config_payload.expires_after = 300;
            config_payload.value_template = Some("{{ value_json.value }}".to_string());
            config_payload.unique_id = config_payload.name.clone();
            config_payload.suggested_display_precision = Some(0);
            config_payload.native_uom = None;
            config_payload.device = device_info.clone();
            state_payload.value = PayloadValueType::Float(data.power_factor);
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: config_topic,
                payload: Payload::Config(config_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
            if let Err(e) = tx.send(IPCMessage::Outbound(PublishMessage {
                topic: state_topic,
                payload: Payload::CurrentState(state_payload.clone())
            })).await {
                return Err(PZEMError::Misc(format!("Couldn't publish payload to ipc bus: {e}")));
            }
        }
        let _ = sleep(Duration::from_millis(MQTT_POLL_INTERVAL_MILLIS)).await;
    }


}
