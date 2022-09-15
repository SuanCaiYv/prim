mod core;
mod entity;
mod persistence;
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::default())
        .with_target(true)
        .with_max_level(tracing::Level::DEBUG)
        .try_init().unwrap();
    core::net::Server::new("0.0.0.0:8190".to_string(), "127.0.0.1:6379".to_string()).await
        .run().await;
}