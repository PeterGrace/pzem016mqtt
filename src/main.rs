mod mqtt_connection;
mod consts;
mod errors;
mod config;
mod mqtt_poll;
mod payload;
mod ipc;

#[macro_use] extern crate tokio;
#[macro_use] extern crate tracing;

use std::fs;
use crate::config::AppConfig;
use lazy_static::lazy_static;
use std::process;
use std::thread::sleep;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::filter::EnvFilter;
use tokio::sync::{broadcast, mpsc, RwLock, OnceCell};
use pzem016lib::PZEM;
use tokio::sync::mpsc::error::TryRecvError;
use crate::consts::{MPSC_BUFFER_SIZE, POLL_TIME};
use crate::ipc::{IPCMessage, PublishMessage};
use crate::mqtt_connection::MqttConnection;
use crate::mqtt_poll::mqtt_poll_loop;
use crate::payload::{generate_payloads, Payload};


lazy_static! {
    static ref SHUTDOWN: OnceCell<bool> = OnceCell::new();
        //region create SETTINGS static object
    static ref SETTINGS: RwLock<AppConfig> = RwLock::new({
         let cfg_file = match std::env::var("CONFIG_FILE_PATH") {
             Ok(s) => s,
             Err(_e) => { "./config.yaml".to_string()}
         };
        let yaml = fs::read_to_string(cfg_file).unwrap_or_else(|e| {
            die(&format!("Can't read config file: {e}"));
            String::default()
            });
        let gc: AppConfig = match serde_yaml::from_str(&yaml)  {
            Ok(gc) => gc,
            Err(e) => { die(&format!("Couldn't deserialize AppConfig: {e}"));
            AppConfig::default()}
        };
        gc
    });
    //endregion
}

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
//region create mqtt server connection and spawn mqtt thread
    let config = SETTINGS.read().await;
    let mqtt_conn = match MqttConnection::new(
        config
            .mqtt_client_id
            .clone()
            .unwrap_or("pzem016mqtt".to_string()),
        config.mqtt_server_addr.clone(),
        config.mqtt_server_port.unwrap_or(1883),
        config.mqtt_username.clone(),
        config.mqtt_password.clone(),
    )
        .await
    {
        Ok(m) => m,
        Err(_e) => {
            return die("Couldn't create mqtt connection object: {e}");
        }
    };

    let (tx, mut rx) = mpsc::channel::<IPCMessage>(MPSC_BUFFER_SIZE);
    let (mqtt_tx, mqtt_rx) =mpsc::channel::<IPCMessage>(MPSC_BUFFER_SIZE);
    let (from_mqtt_tx, mut from_mqtt_rx) = mpsc::channel::<IPCMessage>(MPSC_BUFFER_SIZE);
    let (broadcast_tx, _broadcast_rx) = broadcast::channel::<IPCMessage>(16_usize);

    let bcasttx = broadcast_tx.clone();
    let mut mqtt_handler = tokio::task::spawn(async move {
        let _ = mqtt_poll_loop(
            mqtt_conn,
            mqtt_rx,
            bcasttx.clone().subscribe(),
            from_mqtt_tx,
        )
            .await;
    });
    //endregion

    let mut pzem = PZEM::new("10.174.2.48:4196".to_string()).await.expect("couldn't connect to modbus");
    let pzem_tx = tx.clone();
    let mut pzem_pzem = pzem.clone();
    let mut pzem_handler = tokio::task::spawn(async move {
        let _ = generate_payloads(&mut pzem_pzem,pzem_tx).await;
    });

    let _ = ctrlc::set_handler(move || {
        println!("Received Ctrl-C, communicating to threads to stop");
        let _ = SHUTDOWN.set(true);
    });
    loop {
        if let Some(shutting_down) = SHUTDOWN.get() {
                    break;
        }
        // check thread health
        if pzem_handler.is_finished() {
            warn!("pzem_handler was finished, restarting.");
            let my_tx = tx.clone();
            let mut my_pzem = pzem.clone();
            pzem_handler = tokio::task::spawn(async move {
                let _ = generate_payloads(&mut my_pzem, my_tx).await;
            });
        }
        match rx.try_recv() {
            Ok(ipcm) => {
                match ipcm {
                    IPCMessage::Inbound(_) => {}
                    IPCMessage::Outbound(o) => {
                        if let Err(e) = mqtt_tx.send(
                            IPCMessage::Outbound(o)
                        ).await {
                            die(&e.to_string());
                        }
                    }
                    IPCMessage::PleaseReconnect(_, _) => {}
                    IPCMessage::Error(_) => {}
                    IPCMessage::Shutdown => {}
                }
            }
            Err(e) => match e {
                TryRecvError::Empty => {}
                TryRecvError::Disconnected => {
                    error!("We are disconnected!");
                }
            },

        }
        //sleep(tokio::time::Duration::from_millis(100_u64));
    }
}

pub fn die(msg: &str) {
    println!("{}", msg);
    process::exit(1);
}
