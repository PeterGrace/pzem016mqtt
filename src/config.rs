use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub mqtt_server_addr: String,
    pub mqtt_server_port: Option<u16>,
    pub mqtt_client_id: Option<String>,
    pub mqtt_username: Option<String>,
    pub mqtt_password: Option<String>,
    pub devices: Option<Vec<PZEMDevice>>
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct PZEMDevice {
    pub addr: u8,
    pub port: String,
    pub breaker: String,
}
