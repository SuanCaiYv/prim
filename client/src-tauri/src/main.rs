#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::net::TcpStream;

#[tauri::command]
fn connect(arg: &str) -> String {
    let stream = TcpStream::connect("127.0.0.1:8190").unwrap();
    let mut ans = String::from("connected with: ");
    ans.push_str(arg);
    return ans;
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![connect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
