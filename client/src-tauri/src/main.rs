#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use config::conf;
use lib::{
    entity::{Msg, Type},
    net::{
        client::{Client, ClientConfigBuilder, ClientTlsTimeout},
        MsgMpmcSender, MsgMpscReceiver,
    },
};

use lazy_static::lazy_static;
use serde_json::json;
use service::{
    get_kv_ops, get_msg_ops,
    http::{delete, get, post, put, ResponseResult},
};
use tauri::{Manager, Window, Wry};
use tokio::{
    sync::{Mutex, RwLock},
};
use tracing::error;

mod config;
mod service;
mod util;

lazy_static! {
    static ref MSG_SENDER: Arc<RwLock<Option<MsgMpmcSender>>> = Arc::new(RwLock::new(None));
    static ref MSG_RECEIVER: Arc<RwLock<Option<MsgMpscReceiver>>> = Arc::new(RwLock::new(None));
    static ref SIGNAL_TX: Mutex<Option<tokio::sync::mpsc::Sender<u8>>> = Mutex::new(None);
    static ref SIGNAL_RX: Mutex<Option<tokio::sync::mpsc::Receiver<u8>>> = Mutex::new(None);
    static ref CLIENT_HOLDER1: Mutex<Option<Client>> = Mutex::new(None);
    static ref CLIENT_HOLDER2: Mutex<Option<ClientTlsTimeout>> = Mutex::new(None);
}

const CONNECTED: u8 = 1;
const DISCONNECTED: u8 = 2;

static mut CONFIG_PATH: &'static str = "./config.toml";
static mut LOCAL_DATA_DIR: &'static str = ".";

async fn load_signal() {
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    *SIGNAL_TX.lock().await = Some(tx);
    *SIGNAL_RX.lock().await = Some(rx);
}

#[tokio::main]
async fn main() -> tauri::Result<()> {
    load_signal().await;
    tauri::Builder::default()
        .setup(move |app| {
            // let path_resolver = app.path_resolver();
            // let config_path = path_resolver.resolve_resource("config.toml").unwrap();
            // unsafe {
            //     let box_config_path = Box::new(config_path.to_str().unwrap().to_owned());
            //     CONFIG_PATH = Box::leak(box_config_path);
            // }
            // let local_data_dir = path_resolver.app_local_data_dir().unwrap();
            // if !local_data_dir.exists() {
            //     std::fs::create_dir(&local_data_dir).unwrap();
            // }
            // unsafe {
            //     let box_local_data_dir = Box::new(local_data_dir.to_str().unwrap().to_owned());
            //     LOCAL_DATA_DIR = Box::leak(box_local_data_dir);
            // }
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
            save_msg_list,
            save_msg,
            get_msg_list,
            get_msg,
            del_msg_list,
            latest_seq_num,
            http_get,
            http_put,
            http_post,
            http_delete,
        ])
        .run(tauri::generate_context!())?;
    // .expect("error while running tauri application");
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(conf().log_level)
        .try_init()
        .unwrap();
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
    let remote_address = params.address.parse().expect("invalid address");
    client_config
        .with_remote_address(remote_address)
        .with_ipv4_type(remote_address.is_ipv4())
        .with_domain(conf().server.domain.clone())
        .with_cert(conf().server.cert.clone())
        .with_ipv4_type(remote_address.is_ipv4())
        .with_keep_alive_interval(conf().transport.keep_alive_interval)
        .with_max_bi_streams(conf().transport.max_bi_streams);
    let config = client_config.build().unwrap();
    {
        let mut msg_sender = MSG_SENDER.write().await;
        if msg_sender.is_some() {
            msg_sender.as_mut().unwrap().close();
        }
    }
    match params.mode.as_str() {
        "tcp" => {
            let mut client = ClientTlsTimeout::new(config, std::time::Duration::from_millis(3000));
            if let Err(e) = client.run().await {
                return Err(e.to_string());
            }
            let (io_sender, mut io_receiver, _timeout_receiver) = match client
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
            let auth_resp = io_receiver.recv().await.unwrap();
            if auth_resp.typ() != Type::Auth {
                return Err("auth failed".to_string());
            }
            MSG_SENDER.write().await.replace(io_sender);
            MSG_RECEIVER.write().await.replace(io_receiver);
            CLIENT_HOLDER2.lock().await.replace(client);
            CLIENT_HOLDER1.lock().await.take();
        }
        "udp" => {
            let mut client = Client::new(config);
            if let Err(e) = client.run().await {
                error!("client run error: {}", e);
                return Err(e.to_string());
            }
            let (io_sender, mut io_receiver) = match client
                .io_channel_token(
                    params.user_id.parse::<u64>().unwrap(),
                    params.user_id.parse::<u64>().unwrap(),
                    params.node_id,
                    &params.token,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("build connection failed: {}", e);
                    return Err(e.to_string());
                }
            };
            let auth_resp = io_receiver.recv().await.unwrap();
            if auth_resp.typ() != Type::Auth {
                return Err("auth failed".to_string());
            }
            let auth_msg = Msg::auth(
                params.user_id.parse::<u64>().unwrap(),
                0,
                params.node_id,
                &params.token,
            );
            let auth_msg = Arc::new(auth_msg);
            if let Err(e) = client.new_net_streams(auth_msg.clone()).await {
                error!("build stream failed: {}", e);
                return Err(e.to_string());
            };
            io_receiver.recv().await.unwrap();
            if let Err(e) = client.new_net_streams(auth_msg.clone()).await {
                error!("build stream failed: {}", e);
                return Err(e.to_string());
            }
            io_receiver.recv().await.unwrap();
            if let Err(e) = client.new_net_streams(auth_msg.clone()).await {
                error!("build stream failed: {}", e);
                return Err(e.to_string());
            }
            io_receiver.recv().await.unwrap();
            MSG_SENDER.write().await.replace(io_sender);
            MSG_RECEIVER.write().await.replace(io_receiver);
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
    let tx = &(*SIGNAL_TX.lock().await);
    let tx = tx.as_ref().unwrap();
    if let Err(e) = tx.send(DISCONNECTED).await {
        return Err(e.to_string());
    }
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
                    {
                        // todo bug fix
                        msg_receiver = MSG_RECEIVER.write().await.take().unwrap();
                    }
                    let window = window.clone();
                    tokio::spawn(async move {
                        loop {
                            let msg = msg_receiver.recv().await;
                            match msg {
                                Some(msg) => {
                                    window.emit("recv", msg.as_slice()).unwrap();
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    });
                }
                DISCONNECTED => {
                    CLIENT_HOLDER1.lock().await.take();
                    CLIENT_HOLDER2.lock().await.take();
                }
                _ => {}
            }
        }
    });
}

#[tauri::command]
async fn set_kv(
    key: String,
    val: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let db = get_kv_ops().await;
    match db.set(&key, &val).await {
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

#[tauri::command]
async fn get_kv(key: String) -> std::result::Result<serde_json::Value, String> {
    let db = get_kv_ops().await;
    match db.get(&key).await {
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

#[tauri::command]
async fn del_kv(key: String) -> std::result::Result<serde_json::Value, String> {
    let db = get_kv_ops().await;
    match db.del(&key).await {
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
struct SaveMsgList {
    msg_list: Vec<Vec<u8>>,
}

#[tauri::command]
async fn save_msg_list(params: SaveMsgList) -> std::result::Result<(), String> {
    let db = get_msg_ops().await;
    let arr = params
        .msg_list
        .iter()
        .map(|body| Msg(body.clone()))
        .collect::<Vec<Msg>>();
    match db.insert_or_update_list(arr).await {
        Ok(_) => {}
        Err(e) => {
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct SaveMsg {
    msg: Vec<u8>,
}

#[tauri::command]
async fn save_msg(params: SaveMsg) -> std::result::Result<(), String> {
    let db = get_msg_ops().await;
    match db.insert_or_update(Msg(params.msg)).await {
        Ok(_) => {}
        Err(e) => {
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GetMsgList {
    user_id: String,
    peer_id: String,
    seq_num_from: String,
    seq_num_to: String,
}

#[tauri::command]
async fn get_msg_list(params: GetMsgList) -> std::result::Result<Vec<Vec<u8>>, String> {
    let db = get_msg_ops().await;
    match db
        .find_list(
            params.user_id.parse().unwrap(),
            params.peer_id.parse().unwrap(),
            params.seq_num_from.parse().unwrap(),
            params.seq_num_to.parse().unwrap(),
        )
        .await
    {
        Ok(v) => match v {
            Some(v) => {
                return Ok(v.iter().map(|v| v.0.clone()).collect());
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
    user_id: String,
    peer_id: String,
    seq_num: String,
}

#[tauri::command]
async fn get_msg(params: GetMsg) -> std::result::Result<Vec<u8>, String> {
    let db = get_msg_ops().await;
    match db
        .select(
            params.user_id.parse().unwrap(),
            params.peer_id.parse().unwrap(),
            params.seq_num.parse().unwrap(),
        )
        .await
    {
        Ok(v) => match v {
            Some(v) => {
                return Ok(v.0);
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LatestSeqNumParams {
    user_id: u64,
    peer_id: u64,
}

#[tauri::command]
pub(crate) async fn latest_seq_num(params: LatestSeqNumParams) -> std::result::Result<u64, String> {
    let db = get_msg_ops().await;
    match db.latest_seq_num(params.user_id, params.peer_id).await {
        Ok(v) => match v {
            Some(v) => {
                return Ok(v);
            }
            None => {
                return Ok(0);
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }
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
    uri: String,
    query: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
    body: Option<serde_json::Value>,
}

#[tauri::command]
async fn http_put(params: HttpPutParams) -> std::result::Result<ResponseResult, String> {
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
    match put(
        &params.host,
        &params.uri,
        query.as_object().unwrap_or_else(|| &empty_map),
        headers.as_object().unwrap_or_else(|| &empty_map),
        params.body.as_ref(),
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
struct HttpPostParams {
    host: String,
    uri: String,
    query: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
    body: Option<serde_json::Value>,
}

#[tauri::command]
async fn http_post(params: HttpPostParams) -> std::result::Result<ResponseResult, String> {
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
    match post(
        &params.host,
        &params.uri,
        query.as_object().unwrap_or_else(|| &empty_map),
        headers.as_object().unwrap_or_else(|| &empty_map),
        params.body.as_ref(),
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
struct HttpDeleteParams {
    host: String,
    uri: String,
    query: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
}

#[tauri::command]
async fn http_delete(params: HttpDeleteParams) -> std::result::Result<ResponseResult, String> {
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
    match delete(
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
