use std::io::Result;
use std::sync::Arc;
use ahash::AHashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{oneshot, RwLock};
use tokio::select;
use tracing::{info, debug, warn, error};

use crate::entity::msg;
use crate::{Msg, util};

const BODY_BUF_LENGTH: usize = 1 << 16;

pub type MsgMap = Arc<RwLock<AHashMap<u64, oneshot::Sender<Msg>>>>;

pub async fn listen(host: String, port: i32) -> Result<()> {
    let address = format!("{}:{}", host, port);
    let mut map = Arc::new(RwLock::new(AHashMap::new()));
    let mut tcp_connection = TcpListener::bind(address).await?;
    loop {
        let map_clone = map.clone();
        let (stream, _) = tcp_connection.accept().await.unwrap();
        debug!("new connection: {}", stream.peer_addr().unwrap());
        tokio::spawn(async move {
            if let Err(e) = handler(stream, map_clone).await {
                error!("{}", e);
            }
        });
    }
}

async fn handler(mut stream: TcpStream, connection_map: MsgMap) -> Result<()> {
    // 头部缓冲区数组
    let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
    // 消息载体缓冲区数组，最多支持4096个汉字，只要没有舔狗发小作文还是够用的
    let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
    // 缓冲区切片引用
    let mut head_buf = &mut (*head);
    let mut body_buf = &mut (*body);
    // 处理第一次发送
    // 等待直到可读
    let (sender, mut receiver) = oneshot::channel();
    if let msg = read_msg(&mut stream, &mut head_buf[..], &mut body_buf[..]).await? {
        // 处理一下第一次连接时的用户和连接映射关系
        {
            let mut write_guard = connection_map.write().await;
            (*write_guard).insert(msg.head.sender, sender);
        }
    }
    loop {
        select! {
            readable = stream.readable() => {
                if let msg = read_msg(&mut stream, &mut head_buf[..], &mut body_buf[..]).await? {}
            }
            msg = (&mut receiver) => {
                if let Ok(msg) = msg {
                    stream.write(msg.as_bytes().as_slice()).await?;
                    stream.flush().await?;
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

async fn read_msg(stream: &mut TcpStream, head_buf: &mut [u8], body_buf: &mut [u8]) -> Result<Msg> {
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

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test() {}
}