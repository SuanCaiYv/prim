#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::thread;
use std::time::Duration;
use delay_timer::prelude::{DelayTimer, TaskBuilder};

mod entity;
mod core;
mod util;

fn main() {
    let timer = DelayTimer::new();
    let task = TaskBuilder::default().set_task_id(3).set_frequency_once_by_seconds(3).spawn_routine(|| {
        println!("aaa")
    });
    let _ = timer.add_task(task.unwrap());
    thread::sleep(Duration::from_secs(5));
    // let (sender, receiver) = std::sync::mpsc::sync_channel(1024);
    // let mut client: Option<core::client::Client> = None;
    // tauri::Builder::default()
    //     .setup(move |app| {
    //         app.listen_global("send-msg", |event| {
    //             client.as_mut().unwrap().write(&mut msg::Msg::default());
    //         });
    //         app.listen_global("connect", move |event| {
    //             client = Some(core::client::Client::connect(event.payload().unwrap().to_string(), sender));
    //         });
    //         app.listen_global("close", |event| {
    //         });
    //         std::thread::spawn(move || {
    //             loop {
    //                 if let Ok(msg) = receiver.recv() {
    //                     app.emit_all("recv-msg", msg)
    //                 }
    //             }
    //         });
    //         Ok(())
    //     })
    //     .run(tauri::generate_context!())
    //     .expect("error while running tauri application");
}
