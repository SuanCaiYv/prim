use std::time::Duration;

mod core;
mod entity;
mod persistence;
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::DEBUG)
        .try_init().unwrap();
    core::net::Server::new("127.0.0.1:8190".to_string(), "127.0.0.1:6379".to_string()).await.run().await;
    // core::mock::Client::run("127.0.0.1:8190".to_string(), 1, 2).await;
    // core::mock::Client::run("127.0.0.1:8190".to_string(), 2, 1).await;
    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
}