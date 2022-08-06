use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use ahash::AHashMap;
use byteorder::ByteOrder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{RwLock};
use tracing::{info, debug, warn, error, instrument};
use crate::entity::msg;
use crate::Msg;

pub async fn run(host: String, port: i32) -> std::io::Result<()> {
    let address = format!("{}:{}", host, port);
    let mut connection_map = Arc::new(RwLock::new(AHashMap::new()));
    let mut tcp_connection = TcpListener::bind(address).await?;
    loop {
        let (stream, _) = tcp_connection.accept().await.unwrap();
        info!("new connection");
        let map = connection_map.clone();
        tokio::spawn(async move {
            handle(stream, map).await;
        });
    }
}

const BODY_BUF_LENGTH: usize = 1 << 16;

async fn handle(mut stream: TcpStream, mut connection_map: Arc<RwLock<AHashMap<u64, TcpStream>>>) {
    let mut head_buf = &mut [0;msg::HEAD_LEN];
    // 4096个汉字，只要没有舔狗发小作文还是够用的
    let mut body_buf = &mut [0;BODY_BUF_LENGTH];
    loop {
        // 等待直到可读
        if let Err(e) = stream.readable().await {
            error!("{:?}", e);
            stream.shutdown().await.unwrap();
            break;
        }
        if let Ok(readable_size) = stream.read(&mut head_buf[..]).await {
            if readable_size == 0 {
                info!("connection closed");
                stream.shutdown().await.unwrap();
                break;
            }
            if readable_size != msg::HEAD_LEN {
                error!("read head error");
                continue;
            }
            let mut head = msg::Head::from(&head_buf[..]);
            modify_timestamp(&mut head);
            debug!("{:?}", head);
            if let Ok(_) = stream.read(&mut body_buf[0..head.length as usize]).await {} else {
                error!("read body error");
                break;
            }
            let length = head.length;
            let msg = Msg {
                head,
                payload: Vec::from(&body_buf[0..length as usize]),
            };
            debug!("{:?}", msg);
        } else {
            error!("read head error");
            stream.shutdown().await.expect("shutdown failed");
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

fn modify_timestamp(head: &mut msg::Head) {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let millis = since_the_epoch.as_millis();
    head.timestamp = millis as u64;
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;
    use crate::Msg;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test() {
        tokio::spawn(async move {
            super::run("127.0.0.1".to_string(), 8190).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect("127.0.0.1:8190").await.unwrap();
            let msg = Msg::default();
            let bytes = msg.as_bytes();
            stream.write(bytes.as_slice()).await.unwrap();
            stream.flush().await.unwrap();
            println!("run1");
        });
    }
}