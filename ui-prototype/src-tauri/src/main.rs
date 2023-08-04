#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use config::{conf, load_config};
use lib::{
    entity::{Msg, Type, GROUP_ID_THRESHOLD},
    net::client::ClientConfigBuilder,
};

use lazy_static::lazy_static;
use lib_net_tokio::net::{
    client::{Client, ClientTcp},
    MsgMpscReceiver, MsgSender,
};
use regex::Regex;
use serde_json::json;
use service::{
    get_kv_ops, get_msg_ops,
    http::{delete, get, post, put, ResponseResult},
};
use tauri::{Manager, Window, Wry};
use tokio::sync::{Mutex, RwLock};
use tracing::error;

mod config;
mod service;
mod util;

lazy_static! {
    static ref MSG_SENDER: Arc<RwLock<Option<MsgSender>>> = Arc::new(RwLock::new(None));
    static ref MSG_RECEIVER: Arc<RwLock<Option<MsgMpscReceiver>>> = Arc::new(RwLock::new(None));
    static ref SIGNAL_TX: Mutex<Option<tokio::sync::mpsc::Sender<u8>>> = Mutex::new(None);
    static ref SIGNAL_RX: Mutex<Option<tokio::sync::mpsc::Receiver<u8>>> = Mutex::new(None);
    static ref CLIENT_HOLDER1: Mutex<Option<Client>> = Mutex::new(None);
    static ref CLIENT_HOLDER2: Mutex<Option<ClientTcp>> = Mutex::new(None);
}

const CONNECTED: u8 = 1;
const DISCONNECTED: u8 = 2;

static mut LOCAL_DATA_DIR: &'static str = ".";

async fn load_signal() {
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    *SIGNAL_TX.lock().await = Some(tx);
    *SIGNAL_RX.lock().await = Some(rx);
}

#[tokio::main]
async fn main() -> tauri::Result<()> {
    load_signal().await;
    // load_config("./config.toml");
    tauri::Builder::default()
        .setup(move |app| {
            let path_resolver = app.path_resolver();
            let config_path = path_resolver.resolve_resource("config.toml").unwrap();
            load_config(config_path.to_str().unwrap());
            let local_data_dir = path_resolver.app_local_data_dir().unwrap();
            if !local_data_dir.exists() {
                std::fs::create_dir(&local_data_dir).unwrap();
            }
            unsafe {
                let box_local_data_dir = Box::new(local_data_dir.to_str().unwrap().to_owned());
                LOCAL_DATA_DIR = Box::leak(box_local_data_dir);
            }
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
            test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(conf().log_level)
        .try_init()
        .unwrap();
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct TestParams {
    val: serde_json::Value,
}

#[tauri::command]
async fn test(params: TestParams) -> std::result::Result<String, String> {
    println!("{:?}", preparse(params.val));
    Ok("ok".to_owned())
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ConnectParams {
    address: serde_json::Value,
    token: serde_json::Value,
    mode: serde_json::Value,
    user_id: serde_json::Value,
    node_id: serde_json::Value,
}

#[tauri::command]
async fn connect(params: ConnectParams) -> std::result::Result<(), String> {
    let mut client_config = ClientConfigBuilder::default();
    let remote_address = params
        .address
        .as_str()
        .unwrap()
        .parse()
        .expect("invalid address");
    let token = params.token.as_str().unwrap();
    let mode = params.mode.as_str().unwrap();
    let user_id = preparse(params.user_id).as_u64().unwrap();
    let node_id = params.node_id.as_u64().unwrap();
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
        match msg_sender.take() {
            Some(sender) => {
                sender.close();
            }
            None => {}
        }
    }
    match mode {
        "tcp" => {
            let mut client = ClientTcp::new(config);
            if let Err(e) = client.run().await {
                return Err(e.to_string());
            }
            let (io_sender, mut io_receiver) = match client
                .io_channel_token(user_id, user_id, node_id as u32, token)
                .await
            {
                Ok(v) => v,
                Err(e) => return Err(e.to_string()),
            };
            let auth_resp = io_receiver.recv().await.unwrap();
            if auth_resp.typ() != Type::Auth {
                return Err("auth failed".to_string());
            }
            MSG_SENDER
                .write()
                .await
                .replace(MsgSender::Server(io_sender));
            MSG_RECEIVER.write().await.replace(io_receiver);
            CLIENT_HOLDER2.lock().await.replace(client);
            CLIENT_HOLDER1.lock().await.take();
        }
        "udp" => {
            let max_connections = config.max_bi_streams;
            let mut client = Client::new(config);
            if let Err(e) = client.run().await {
                error!("client run error: {}", e);
                return Err(e.to_string());
            }
            let (io_sender, mut io_receiver) = match client
                .io_channel_token(user_id, user_id, node_id as u32, token)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("build connection failed: {}", e);
                    return Err(e.to_string());
                }
            };
            for _ in 0..max_connections {
                let auth_resp = io_receiver.recv().await.unwrap();
                if auth_resp.typ() != Type::Auth {
                    return Err("auth failed".to_string());
                }
            }
            MSG_SENDER
                .write()
                .await
                .replace(MsgSender::Client(io_sender));
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
    let val = preparse(val);
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
    user_id: serde_json::Value,
    peer_id: serde_json::Value,
    seq_num_from: serde_json::Value,
    seq_num_to: serde_json::Value,
}

#[tauri::command]
async fn get_msg_list(params: GetMsgList) -> std::result::Result<Vec<Vec<u8>>, String> {
    let user_id = preparse(params.user_id).as_u64().unwrap();
    let peer_id = preparse(params.peer_id).as_u64().unwrap();
    let seq_num_from = preparse(params.seq_num_from).as_u64().unwrap();
    let seq_num_to = preparse(params.seq_num_to).as_u64().unwrap();
    let db = get_msg_ops().await;
    if peer_id >= GROUP_ID_THRESHOLD {
        match db
            .find_list(peer_id, peer_id, seq_num_from, seq_num_to)
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
    } else {
        match db
            .find_list(user_id, peer_id, seq_num_from, seq_num_to)
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
    };
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GetMsg {
    user_id: serde_json::Value,
    peer_id: serde_json::Value,
    seq_num: serde_json::Value,
}

#[tauri::command]
async fn get_msg(params: GetMsg) -> std::result::Result<Vec<u8>, String> {
    let user_id = preparse(params.user_id).as_u64().unwrap();
    let peer_id = preparse(params.peer_id).as_u64().unwrap();
    let seq_num = preparse(params.seq_num).as_u64().unwrap();
    let db = get_msg_ops().await;
    match db.select(user_id, peer_id, seq_num).await {
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
    user_id: serde_json::Value,
    peer_id: serde_json::Value,
    seq_num_list: Vec<u64>,
}

#[tauri::command]
async fn del_msg_list(params: DelMsgList) -> std::result::Result<(), String> {
    let user_id = preparse(params.user_id).as_u64().unwrap();
    let peer_id = preparse(params.peer_id).as_u64().unwrap();
    let db = get_msg_ops().await;
    match db
        .delete_list(user_id, peer_id, params.seq_num_list.as_slice())
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
    user_id: serde_json::Value,
    peer_id: serde_json::Value,
}

#[tauri::command]
pub(crate) async fn latest_seq_num(params: LatestSeqNumParams) -> std::result::Result<u64, String> {
    let user_id = preparse(params.user_id).as_u64().unwrap();
    let peer_id = preparse(params.peer_id).as_u64().unwrap();
    let db = get_msg_ops().await;
    if peer_id >= GROUP_ID_THRESHOLD {
        match db.latest_seq_num(peer_id, peer_id).await {
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
    } else {
        match db.latest_seq_num(user_id, peer_id).await {
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
    };
}

#[inline(always)]
fn preparse(obj: serde_json::Value) -> serde_json::Value {
    match obj {
        serde_json::Value::String(v) => {
            if v.ends_with('n') {
                let v = v.trim_end_matches('n');
                let regex = Regex::new(r"^\d+$").unwrap();
                if regex.is_match(v) {
                    match v.parse::<i64>() {
                        Ok(v) => {
                            return json!(v);
                        }
                        Err(_) => {}
                    }
                    match v.parse::<u64>() {
                        Ok(v) => {
                            return json!(v);
                        }
                        Err(_) => {}
                    }
                    match v.parse::<f64>() {
                        Ok(v) => {
                            return serde_json::Value::Number(
                                serde_json::Number::from_f64(v as f64).unwrap(),
                            );
                        }
                        Err(_) => {}
                    }
                }
            }
            return serde_json::Value::String(v);
        }
        serde_json::Value::Array(v) => {
            let mut ret = vec![];
            for v in v {
                ret.push(preparse(v));
            }
            return serde_json::Value::Array(ret);
        }
        serde_json::Value::Object(v) => {
            let mut ret = serde_json::Map::new();
            for (k, v) in v {
                ret.insert(k, preparse(v));
            }
            return serde_json::Value::Object(ret);
        }
        serde_json::Value::Number(v) => {
            return serde_json::Value::Number(v);
        }
        serde_json::Value::Bool(v) => {
            return serde_json::Value::Bool(v);
        }
        serde_json::Value::Null => {
            return serde_json::Value::Null;
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
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let headers = match params.headers {
        Some(v) => preparse(v),
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
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let headers = match params.headers {
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let body = match params.body {
        Some(body) => Some(preparse(body)),
        None => None,
    };
    let empty_map = serde_json::Map::new();
    match put(
        &params.host,
        &params.uri,
        query.as_object().unwrap_or_else(|| &empty_map),
        headers.as_object().unwrap_or_else(|| &empty_map),
        body.as_ref(),
    )
    .await
    {
        Ok(v) => {
            return Ok(v);
        }
        Err(e) => {
            error!("http_put error: {}", e);
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
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let headers = match params.headers {
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let body = match params.body {
        Some(body) => Some(preparse(body)),
        None => None,
    };
    let empty_map = serde_json::Map::new();
    match post(
        &params.host,
        &params.uri,
        query.as_object().unwrap_or_else(|| &empty_map),
        headers.as_object().unwrap_or_else(|| &empty_map),
        body.as_ref(),
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
        Some(v) => preparse(v),
        None => {
            json!(null)
        }
    };
    let headers = match params.headers {
        Some(v) => preparse(v),
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
