use std::{sync::Arc, println, time::Duration};
use persistence::persistence_sequence_number_threshold;
use thread_local::ThreadLocal;
use tokio::io::AsyncReadExt;
use tracing::Level;

mod config;
mod persistence;
mod scheduler;
mod service;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(Level::DEBUG)
        .try_init()
        .unwrap();
    let file_tl = Arc::new(ThreadLocal::new());
    for i in 0..33 {
        _ = persistence_sequence_number_threshold(&file_tl, i % 3, i % 3 + 1, i).await;
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!("ok");
    let mut buf = [0u8; 24];
    let mut dir = std::fs::read_dir("./").unwrap();
    let file = dir.find(|e| {
        let f = e.as_ref().unwrap();
        f.file_name().to_str().unwrap().starts_with("seqnum-")
    });
    println!("{:?}", file);
    if let Some(file) = file {
        let file = file.unwrap();
        let mut file = tokio::fs::File::open(file.path()).await.unwrap();
        while let Ok(_) = file.read_exact(&mut buf).await {
            println!("{:?}", buf);
        }
    }
}
