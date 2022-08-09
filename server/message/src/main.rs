use std::borrow::BorrowMut;
use std::ops::Deref;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info, warn, error};
use crate::entity::msg::Msg;
use crate::core::net;
use crate::entity::msg;

mod util;
mod entity;
mod core;
mod persistence;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::DEBUG)
        .try_init().unwrap();
    tokio::spawn(async move {
        net::listen("127.0.0.1".to_string(), 8190).await;
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect("127.0.0.1:8190").await.unwrap();
        let mut read = Arc::new(stream);
        let mut write = read.clone();
        tokio::spawn(async move {
            let mut read = read;
            // 头部缓冲区数组
            let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
            // 消息载体缓冲区数组，最多支持4096个汉字，只要没有舔狗发小作文还是够用的
            let mut body: Box<[u8; 1 << 16]> = Box::new([0; 1 << 16]);
            // 缓冲区切片引用
            let mut head_buf = &mut (*head);
            let mut body_buf = &mut (*body);
            loop {
                read_msg(read, &mut head_buf[..], &mut body_buf[..]).await;
            }
        });
        stream.write(msg::Msg::text_str(1, 2, "hello").as_bytes().as_slice()).await.unwrap();
    });
    tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect("127.0.0.1:8190").await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    Ok(())
}

async fn read_msg(stream: &mut TcpStream, head_buf: &mut [u8], body_buf: &mut [u8]) -> std::io::Result<Msg> {
    return if let Ok(readable_size) = stream.read(head_buf).await {
        if readable_size == 0 {
            debug!("connection closed");
            stream.shutdown().await?;
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "connection closed"));
        }
        if readable_size != msg::HEAD_LEN {
            error!("read head error");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "read head error"));
        }
        let mut head = msg::Head::from(&head_buf[..]);
        head.timestamp = util::base::timestamp();
        debug!("{:?}", head);
        if let body_length = stream.read(&mut body_buf[0..head.length as usize]).await? {
            if body_length != head.length as usize {
                error!("read body error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "read body error"));
            }
        }
        let length = head.length;
        let msg = Msg {
            head,
            payload: Vec::from(&body_buf[0..length as usize]),
        };
        debug!("{:?}", msg);
        Ok(msg)
    } else {
        error!("read head error");
        stream.shutdown().await?;
        Err(std::io::Error::new(std::io::ErrorKind::Other, "read head error"))
    }
}
