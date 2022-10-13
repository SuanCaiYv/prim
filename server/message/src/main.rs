use std::time::Duration;
use tracing::info;

mod cache;
mod config;
mod core;
mod entity;
mod error;
mod joy;
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::default())
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .unwrap();
    info!("prim server running");
    println!("{}", joy::banner());
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        core::mock().await.unwrap();
    });
    let _ = core::start().await;
}
