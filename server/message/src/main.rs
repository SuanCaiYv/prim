use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::info;
use crate::entity::msg::Msg;
use crate::core::connection;

mod util;
mod entity;
mod core;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::DEBUG)
        .try_init().unwrap();
    tokio::spawn(async move {
        connection::run("127.0.0.1".to_string(), 8190).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect("127.0.0.1:8190").await.unwrap();
        let msg = Msg::default();
        let bytes = msg.as_bytes();
        stream.write(bytes.as_slice()).await.unwrap();
        stream.flush().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        let msg1 = Msg::default();
        let bytes1 = msg1.as_bytes();
        stream.write(bytes1.as_slice()).await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    Ok(())
}
