#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::str::FromStr;
use tauri::Manager;
use log::{info, error, debug};
use crate::entity::msg;

mod entity;
mod core;
mod util;

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .setup(move |app| {
            let client = std::sync::Arc::new(std::sync::Mutex::new(None));
            let sender = std::sync::Arc::new(std::sync::Mutex::new(None));
            let mut receiver = std::sync::Arc::new(std::sync::Mutex::new(None));
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let main_window = app.get_window("main").unwrap();

            let client_c = client.clone();
            let sender_c = sender.clone();
            let receiver_c = receiver.clone();
            main_window.listen("connect", move |event| {
                let mut address = String::new();
                if let Some(address0) = event.payload() {
                    address.push_str(address0);
                } else {
                    error!("No address provided");
                    return;
                }
                if let Ok(mut client0) = core::client::Client::connect(address) {
                    client0.run();
                    {
                        *(sender_c.lock().unwrap()) = Some(client0.write())
                    }
                    {
                        *(receiver_c.lock().unwrap()) = Some(client0.read())
                    }
                    {
                        *(client_c.lock().unwrap()) = Some(client0);
                    }
                } else {
                    error!("failed to connect server");
                }
                tx.send(()).unwrap();
            });
            let client_c = client.clone();
            main_window.listen("heartbeat", move |event| {
                let mut sender: u64 = 0;
                if let Some(payload) = event.payload() {
                    if let Ok(sender0) = u64::from_str(payload) {
                        sender = sender0;
                    } else {
                        error!("failed to parse sender id");
                        return;
                    }
                } else {
                    error!("no sender provided");
                    return;
                }
                u64::from_str(event.payload().unwrap()).unwrap();
                if let Ok(mut lock) = client_c.lock() {
                    (*lock).as_mut().unwrap().heartbeat(sender);
                } else {
                    error!("dead lock detected!")
                }
            });
            main_window.listen("close", move |event| {
                if let Ok(mut lock) = client.lock() {
                    (*lock).as_mut().unwrap().close();
                } else {
                    error!("dead lock detected!")
                }
            });
            main_window.listen("send-msg", move |event| {
                if let Some(payload) = event.payload() {
                    if let Ok(mut lock) = sender.lock() {
                        if let Err(_) = (*lock).as_mut().unwrap().send(msg::Msg::from(payload.as_bytes())) {
                            error!("failed to send message");
                        }
                    } else {
                        error!("dead lock detected!")
                    }
                } else {
                    error!("no msg provided");
                }
            });
            std::thread::spawn(move || {
                rx.recv().unwrap();
                let recv = &mut receiver;
                // 独占一把锁
                loop {
                    if let Ok(msg) = (*(recv.lock().unwrap())).as_mut().unwrap().recv() {
                        main_window.emit("recv-msg", msg).unwrap();
                    } else {
                        error!("read sender closed");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
