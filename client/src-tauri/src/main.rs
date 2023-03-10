#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use config::CONFIG;
use lib::{
    entity::Msg,
    net::{
        client::{Client2Timeout, ClientConfigBuilder, ClientTimeout},
        OuterReceiver, OuterSender,
    },
};

use lazy_static::lazy_static;
use tauri::{Manager, Window, Wry};
use tokio::{
    select,
    sync::{Mutex, RwLock},
};

mod config;
mod service;
mod util;

lazy_static! {
    static ref MSG_SENDER: Arc<RwLock<Option<OuterSender>>> = Arc::new(RwLock::new(None));
    static ref MSG_RECEIVER: Arc<RwLock<Option<OuterReceiver>>> = Arc::new(RwLock::new(None));
    static ref TIMEOUT_RECEIVER: Arc<RwLock<Option<OuterReceiver>>> = Arc::new(RwLock::new(None));
    static ref SIGNAL_TX: Mutex<Option<tokio::sync::mpsc::Sender<u8>>> = Mutex::new(None);
    static ref SIGNAL_RX: Mutex<Option<tokio::sync::mpsc::Receiver<u8>>> = Mutex::new(None);
    static ref CLIENT_HOLDER1: Mutex<Option<ClientTimeout>> = Mutex::new(None);
    static ref CLIENT_HOLDER2: Mutex<Option<Client2Timeout>> = Mutex::new(None);
}

const CONNECTED: u8 = 1;

async fn load_signal() {
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    *SIGNAL_TX.lock().await = Some(tx);
    *SIGNAL_RX.lock().await = Some(rx);
}

#[tokio::main]
async fn main() -> tauri::Result<()> {
    load_signal().await;
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .unwrap();
    tauri::Builder::default()
        .setup(move |app| {
            let window = app.get_window("main").unwrap();
            setup(window);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![connect, send]);
        // .run(tauri::generate_context!())
        // .expect("error while running tauri application");
        Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ConnectParams {
    address: String,
    token: String,
    mode: String,
    user_id: String,
    node_id: u32,
}

#[tauri::command]
async fn connect(params: ConnectParams) -> Result<(), String> {
    let mut client_config = ClientConfigBuilder::default();
    client_config
        .with_remote_address(params.address.parse().expect("invalid address"))
        .with_domain(CONFIG.server.domain.clone())
        .with_cert(CONFIG.server.cert.clone())
        .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams)
        .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
        .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
    let config = client_config.build().unwrap();
    {
        let mut msg_sender = MSG_SENDER.write().await;
        if msg_sender.is_some() {
            msg_sender.as_mut().unwrap().close();
        }
    }
    match params.mode.as_str() {
        "tcp" => {
            let mut client = Client2Timeout::new(config, std::time::Duration::from_millis(3000));
            if let Err(e) = client.run().await {
                return Err(e.to_string());
            }
            let (io_sender, io_receiver, timeout_receiver) = match client
                .io_channel_token(
                    params.user_id.parse::<u64>().unwrap(),
                    params.user_id.parse::<u64>().unwrap(),
                    params.node_id,
                    &params.token,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => return Err(e.to_string()),
            };
            MSG_SENDER.write().await.replace(io_sender);
            MSG_RECEIVER.write().await.replace(io_receiver);
            TIMEOUT_RECEIVER.write().await.replace(timeout_receiver);
            CLIENT_HOLDER2.lock().await.replace(client);
            CLIENT_HOLDER1.lock().await.take();
        }
        "udp" => {
            let mut client =
                ClientTimeout::new(config, std::time::Duration::from_millis(3000), true);
            if let Err(e) = client.run().await {
                return Err(e.to_string());
            }
            if let Err(e) = client.new_net_streams().await {
                return Err(e.to_string());
            };
            if let Err(e) = client.new_net_streams().await {
                return Err(e.to_string());
            }
            if let Err(e) = client.new_net_streams().await {
                return Err(e.to_string());
            }
            let (io_sender, io_receiver, timeout_receiver) = match client
                .io_channel_token(
                    params.user_id.parse::<u64>().unwrap(),
                    params.user_id.parse::<u64>().unwrap(),
                    params.node_id,
                    &params.token,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => return Err(e.to_string()),
            };
            MSG_SENDER.write().await.replace(io_sender);
            MSG_RECEIVER.write().await.replace(io_receiver);
            TIMEOUT_RECEIVER.write().await.replace(timeout_receiver);
            CLIENT_HOLDER1.lock().await.replace(client);
            CLIENT_HOLDER2.lock().await.take();
        }
        _ => {
            return Err("invalid mode".to_string());
        }
    }
    let tx = &(*SIGNAL_TX.lock().await);
    let tx = tx.as_ref().unwrap();
    if let Err(e) = tx.send(CONNECTED).await {
        return Err(e.to_string());
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct SendParams {
    raw: Vec<u8>,
}

#[tauri::command]
async fn send(params: SendParams) -> Result<(), String> {
    let msg = Msg(params.raw);
    println!("{}", msg);
    let msg_sender = MSG_SENDER.read().await;
    match *msg_sender {
        Some(ref sender) => {
            if let Err(e) = sender.send(Arc::new(msg)).await {
                return Err(e.to_string());
            }
        }
        None => {
            return Err("not connected".to_string());
        }
    }
    Ok(())
}

async fn disconnect() -> Result<(), String> {
    Ok(())
}

fn setup(window: Window<Wry>) {
    tokio::spawn(async move {
        let mut signal_rx = SIGNAL_RX.lock().await.take().unwrap();
        loop {
            let signal = signal_rx.recv().await;
            if signal.is_none() {
                break;
            }
            match signal.unwrap() {
                CONNECTED => {
                    let mut msg_receiver;
                    let mut timeout_receiver;
                    {
                        msg_receiver = MSG_RECEIVER.write().await.take().unwrap();
                        timeout_receiver = TIMEOUT_RECEIVER.write().await.take().unwrap();
                    }
                    let window = window.clone();
                    tokio::spawn(async move {
                        loop {
                            select! {
                                msg = msg_receiver.recv() => {
                                    match msg {
                                        Some(msg) => {
                                            window.emit("recv", msg).unwrap();
                                        },
                                        None => {
                                            break;
                                        }
                                    }
                                },
                                timeout = timeout_receiver.recv() => {
                                    match timeout {
                                        Some(timeout) => {
                                            window.emit("timeout", timeout).unwrap();
                                        },
                                        None => {
                                            break;
                                        }
                                    }
                                },
                            }
                        }
                    });
                }
                _ => {}
            }
        }
    });
}
