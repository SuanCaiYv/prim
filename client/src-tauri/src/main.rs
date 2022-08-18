#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use serde::de::Unexpected::Option;
use tauri::Manager;
use crate::entity::msg;

mod entity;
mod core;
mod util;

fn main() {
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
                let mut client0 = core::client::Client::connect(event.payload().unwrap().to_string()).unwrap();
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
                tx.send(()).unwrap();
            });
            main_window.listen("send-msg", move |event| {
                // (*(sender.lock().unwrap())).as_mut().unwrap().send(msg::Msg::from(event.payload().unwrap().as_bytes()));
                (*(sender.lock().unwrap())).as_mut().unwrap().send(msg::Msg::text(1, 0, event.payload().unwrap().to_string())).unwrap();
                println!("{}", event.payload().unwrap());
            });
            main_window.listen("close", move |event| {
                (*(client.lock().unwrap())).as_mut().unwrap().close();
            });
            std::thread::spawn(move || {
                rx.recv().unwrap();
                let recv = &mut receiver;
                loop {
                    {
                        let msg = (*(recv.lock().unwrap())).as_mut().unwrap().recv().unwrap();
                        println!("ccc: {:?}", msg);
                        main_window.emit("recv-msg", msg);
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
