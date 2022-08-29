#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use byteorder::ByteOrder;
use tauri::Manager;
use serde::{Serialize, Deserialize};
use tokio::runtime::Handle;
use tracing::{debug, info, warn, error};
use tracing::field::debug;
use crate::entity::msg;
use crate::msg::Msg;

mod entity;
mod core;
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::DEBUG)
        .try_init().unwrap();
    tauri::Builder::default()
        .setup(move |app| {
            let window = app.get_window("main").unwrap();
            setup(window);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Cmd {
    name: String,
    args: Vec<Vec<u8>>,
}

impl Display for Cmd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.name == "send-msg" {
            write!(f, "Cmd [ name: send-msg, args: {} ]", Msg::from(&self.args[0]))
        } else {
            write!(f, "Cmd [ name: {}, args: {} ]", self.name, String::from_utf8_lossy(&(self.args[0])))
        }
    }
}

impl Cmd {
    fn connect_result(result: bool) -> Self {
        let mut args = Vec::with_capacity(1);
        args.push(Vec::from(true.to_string()));
        Self {
            name: String::from("connect-result"),
            args,
        }
    }

    fn recv_msg(msg: &msg::Msg) -> Self {
        let mut args = Vec::with_capacity(1);
        args.push(msg.as_bytes());
        Self {
            name: String::from("recv-msg"),
            args,
        }
    }

    fn text_str(text: &'static str) -> Self {
        let mut args = Vec::with_capacity(1);
        args.push(Vec::from(text));
        Self {
            name: String::from("text-str"),
            args,
        }
    }

    fn from_payload(payload: &str) -> Self {
        let cmd: Result<Cmd, serde_json::Error> = serde_json::from_str(payload);
        if let Err(_) = cmd {
            return Self {
                name: String::from(""),
                args: Vec::new(),
            }
        } else {
            cmd.unwrap()
        }
    }
}

fn setup(window1: tauri::window::Window<tauri::Wry>) {
    let mut cmd_unlisten: Option<tauri::EventHandler> = None;
    let window2 = window1.clone();
    window1.listen("test", move |event| {
        if let Ok(rt) = Handle::try_current() {
            println!("{:?}", rt);
        }
    });
    window1.listen("connect", move |event| {
        if let Some(f) = cmd_unlisten {
            window2.unlisten(f)
        }
        let address = event.payload();
        if let None = address {
            window2.emit("cmd-res", Cmd::connect_result(false));
            error!("need address provided");
            return;
        }
        let address = address.unwrap().to_string();
        let window3 = window2.clone();
        tauri::async_runtime::spawn(async move {
            let client = core::client::Client::connect(address).await;
            if let Err(_) = client {
                error!("can't connect to server");
                window3.emit("cmd-res", Cmd::connect_result(false));
                return;
            }
            let mut client = client.unwrap();
            client.run();
            let data_in = client.data_in();
            let mut data_out = client.data_out();
            debug!("new connection established");
            let a = window3.emit("cmd-res", Cmd::connect_result(true));
            debug!("{:?}", a);
            let window4 = window3.clone();
            let client = std::sync::Arc::new(tokio::sync::Mutex::new(client));
            if let Some(unlisten) = cmd_unlisten {
                window3.unlisten(unlisten);
            }
            let listen_id = window3.listen("cmd", move |event| {
                tauri::async_runtime::spawn(async move {});
                let payload = event.payload();
                if let None = payload {
                    return;
                }
                let payload = payload.unwrap();
                let cmd = Cmd::from_payload(payload);
                println!("{}", cmd);
                if cmd.name.is_empty() {
                    window4.emit("cmd-res", Cmd::text_str("parse failed"));
                    return;
                }
                // 官方的另一个方式的异步支持也是针对每一次调用spawn一个上下文去处理，所以这里暂时不考虑性能损失
                // 此外作者给我的建议也是这样
                // 作者给出的另一个方式是block_on()，但是这个方法会导致tokio运行时报错，因为无论你怎么调用block_on()
                // 哪怕是在新的runtime执行也罢，最终都会由当前runtime推动，除非你直接把tokio::main进行替换。
                // 而此时会直接阻塞当前runtime，造成tokio panic。所以这里选择spawn形式。
                match cmd.name.as_str() {
                    "heartbeat" => {
                        let sender_id = byteorder::BigEndian::read_u64(cmd.args[0].as_slice());
                        let client = client.clone();
                        tauri::async_runtime::spawn(async move {
                            let lock = client.lock().await;
                            (*lock).heartbeat(sender_id);
                        });
                    },
                    "close" => {
                        let client = client.clone();
                        tauri::async_runtime::spawn(async move {
                            let lock = client.lock().await;
                            (*lock).close().await;
                        });
                    },
                    "send-msg" => {
                        let data_in = data_in.clone();
                        let msg = msg::Msg::from(&cmd.args[0]);
                        debug!("{}", msg);
                        tauri::async_runtime::spawn(async move {
                            let _ = data_in.send(msg).await;
                        });
                    },
                    _ => {}
                };
            });
            tauri::async_runtime::spawn(async move {
                let mut data_out = &mut data_out;
                loop {
                    let msg = data_out.recv().await;
                    if let None = msg {
                        return;
                    }
                    let msg = msg.unwrap();
                    window3.emit("cmd-res", Cmd::recv_msg(&msg));
                }
            });
            cmd_unlisten = Some(listen_id)
        });
    });
}

#[cfg(test)]
mod tests {
    use crate::Cmd;
    use crate::msg::Msg;

    #[test]
    fn test() {
        let str = "{\"name\":\"send-msg\",\"args\":[[0,3,1,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,123,0,0,1,130,232,42,45,73,0,0,0,0,0,0,0,0,0,0,98,98,98]]}";
        let cmd = Cmd::from_payload(str);
        println!("{}", cmd);
    }
}
