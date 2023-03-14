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
use serde_json::json;
use service::{
    get_kv_ops, get_msg_ops,
    http::{get, ResponseResult},
};
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
        .invoke_handler(tauri::generate_handler![
            connect,
            disconnect,
            send,
            set_kv,
            get_kv,
            del_kv,
            save_msg,
            get_msg_list,
            get_msg,
            del_msg_list
        ]);
    // .run(tauri::generate_context!())?;
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
async fn connect(params: ConnectParams) -> std::result::Result<(), String> {
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
async fn send(params: SendParams) -> std::result::Result<(), String> {
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

#[tauri::command]
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

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct KVSet {
    key: String,
    val: String,
}

#[tauri::command]
async fn set_kv(params: KVSet) -> std::result::Result<String, String> {
    let db = get_kv_ops().await;
    match db.set(&params.key, &params.val).await {
        Ok(val) => match val {
            Some(v) => {
                return Ok(v);
            }
            None => {
                return Err("not found".to_string());
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct KVGet {
    key: String,
}

#[tauri::command]
async fn get_kv(params: KVGet) -> std::result::Result<String, String> {
    let db = get_kv_ops().await;
    match db.get(&params.key).await {
        Ok(v) => {
            if let Some(v) = v {
                return Ok(v);
            }
            return Err("not found".to_string());
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct KVDelete {
    key: String,
}

#[tauri::command]
async fn del_kv(params: KVDelete) -> std::result::Result<String, String> {
    let db = get_kv_ops().await;
    match db.del(&params.key).await {
        Ok(val) => match val {
            Some(v) => {
                return Ok(v);
            }
            None => {
                return Err("not found".to_string());
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct SaveMsg {
    msg_list: Vec<Msg>,
}

#[tauri::command]
async fn save_msg(params: SaveMsg) -> std::result::Result<(), String> {
    let db = get_msg_ops().await;
    match db.insert_or_update(params.msg_list.as_slice()).await {
        Ok(_) => {}
        Err(e) => {
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GetMsgList {
    user_id: u64,
    peer_id: u64,
    seq_num_from: u64,
    seq_num_to: u64,
}

#[tauri::command]
async fn get_msg_list(params: GetMsgList) -> std::result::Result<Vec<Msg>, String> {
    let db = get_msg_ops().await;
    match db
        .find_list(
            params.user_id,
            params.peer_id,
            params.seq_num_from,
            params.seq_num_to,
        )
        .await
    {
        Ok(v) => match v {
            Some(v) => {
                return Ok(v);
            }
            None => {
                return Ok(vec![]);
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GetMsg {
    user_id: u64,
    peer_id: u64,
    seq_num: u64,
}

#[tauri::command]
async fn get_msg(params: GetMsg) -> std::result::Result<Msg, String> {
    let db = get_msg_ops().await;
    match db
        .select(params.user_id, params.peer_id, params.seq_num)
        .await
    {
        Ok(v) => match v {
            Some(v) => {
                return Ok(v);
            }
            None => {
                return Err("not found".to_string());
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct DelMsgList {
    user_id: u64,
    peer_id: u64,
    seq_num_list: Vec<u64>,
}

#[tauri::command]
async fn del_msg_list(params: DelMsgList) -> std::result::Result<(), String> {
    let db = get_msg_ops().await;
    match db
        .delete_list(
            params.user_id,
            params.peer_id,
            params.seq_num_list.as_slice(),
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct HttpGetParams {
    host: String,
    uri: String,
    query: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
}

#[tauri::command]
async fn http_get(params: HttpGetParams) -> std::result::Result<ResponseResult, String> {
    let query = match params.query {
        Some(v) => v,
        None => {
            json!(null)
        }
    };
    let headers = match params.headers {
        Some(v) => v,
        None => {
            json!(null)
        }
    };
    let empty_map = serde_json::Map::new();
    match get(
        &params.host,
        &params.uri,
        query.as_object().unwrap_or_else(|| &empty_map),
        headers.as_object().unwrap_or_else(|| &empty_map),
    )
    .await
    {
        Ok(v) => {
            return Ok(v);
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct HttpPutParams {
    host: String,
}
