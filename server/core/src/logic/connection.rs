use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use ahash::AHashMap;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{RwLock};

pub async fn bind(host: String, port: i32) -> std::io::Result<()> {
    let address = format!("{}:{}", host, port);
    let mut connection_map = Arc::new(RwLock::new(AHashMap::new()));
    let mut tcp_connection = TcpListener::bind(address).await?;
    loop {
        let mut stream = tcp_connection.accept().await.unwrap();
        let map = connection_map.clone();
        tokio::spawn(async move {
            handle(stream.0, map).await;
        });
    }
}

async fn handle(mut stream: TcpStream, mut connection_map: Arc<RwLock<AHashMap<u64, TcpStream>>>) {
    let mut head_buf: [u8;37] = [0;37];
    // 4096个汉字，只要没有舔狗发小作文还是够用的
    let mut body_buf: [u8;2 << 16] = [0;2 << 16];
    loop {
        if let Ok(_) = stream.read(&mut head_buf).await {
            println!("{}, {}", head_buf[0], head_buf[1]);
            let length = (head_buf[0] + head_buf[1] << 4) as usize;
            println!("{}", length);
            let typ = head_buf[2];
            let sender = head_buf[3] + head_buf[4] << 8 + head_buf[5] << 16 + head_buf[6] << 24;
            let receiver = head_buf[7] + head_buf[8] << 8 + head_buf[9] << 16 + head_buf[10] << 24;
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let millis = since_the_epoch.as_millis();
            head_buf[11] = (millis >> 24) as u8;
            head_buf[12] = (millis >> 16) as u8;
            head_buf[13] = (millis >> 8) as u8;
            head_buf[14] = millis as u8;
            if let Ok(_) = stream.read(&mut body_buf[0..length]).await {} else {
                println!("read body error");
                break;
            }
        } else {
            println!("connection closed");
            break;
        }
    }
}