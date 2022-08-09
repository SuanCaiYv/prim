use std::io::Result;
use std::sync::Arc;
use ahash::AHashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{mpsc, RwLock};
use tokio::select;
use tracing::{info, debug, warn, error};

use crate::entity::msg;
use crate::{Msg, util};
use crate::core::{biz, heartbeat, logic};
use crate::persistence::redis_ops::RedisOps;

const BODY_BUF_LENGTH: usize = 1 << 16;
const MAX_FRIENDS_NUMBER: usize = 1 << 10;

pub type MsgMap = Arc<RwLock<AHashMap<u64, mpsc::Sender<Msg>>>>;
pub type StatusMap = Arc<RwLock<AHashMap<u64, u64>>>;

pub async fn listen(host: String, port: i32) -> Result<()> {
    let address = format!("{}:{}", host, port);
    let mut connection_map = Arc::new(RwLock::new(AHashMap::new()));
    let mut statue_map: StatusMap = Arc::new(RwLock::new(AHashMap::new()));
    let mut redis_ops = RedisOps::connection("127.0.0.1".to_string(), 6379).await;
    let mut tcp_connection = TcpListener::bind(address).await?;
    loop {
        let map1_clone = connection_map.clone();
        let map2_clone = statue_map.clone();
        let redis_ops_clone = redis_ops.clone();
        let (stream, _) = tcp_connection.accept().await.unwrap();
        debug!("new connection: {}", stream.peer_addr().unwrap());
        tokio::spawn(async move {
            if let Err(e) = handler(stream, map1_clone, map2_clone, redis_ops_clone).await {
                error!("{}", e);
            }
        });
    }
}

async fn handler(mut stream: TcpStream, mut connection_map: MsgMap, mut statue_map: StatusMap, mut redis_ops: RedisOps) -> Result<()> {
    // 头部缓冲区数组
    let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
    // 消息载体缓冲区数组，最多支持4096个汉字，只要没有舔狗发小作文还是够用的
    let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
    // 缓冲区切片引用
    let mut head_buf = &mut (*head);
    let mut body_buf = &mut (*body);
    // 处理第一次发送
    // 等待直到可读
    let (sender, mut receiver) = mpsc::channel(MAX_FRIENDS_NUMBER);
    if let msg = read_msg(&mut stream, &mut head_buf[..], &mut body_buf[..]).await? {
        // 处理一下第一次连接时的用户和连接映射关系
        {
            let mut write_guard = connection_map.write().await;
            (*write_guard).insert(msg.head.sender, sender);
        }
    }
    let mut map1 = &mut connection_map;
    let mut map2 = &mut statue_map;
    let mut redis = &mut redis_ops;
    loop {
        select! {
            readable = stream.readable() => {
                if let mut msg = read_msg(&mut stream, &mut head_buf[..], &mut body_buf[..]).await? {
                    if let Some(msg) = heartbeat::work(&mut msg, map2).await {
                        if let Err(e) = stream.write(msg.as_bytes().as_slice()).await {
                            error!("connection closed: {}", e);
                            stream.shutdown().await?;
                            return Ok(());
                        }
                        stream.flush().await?;
                        continue;
                    }
                    if let Some(list) = logic::work(&mut msg, map1, redis).await {
                        for msg in list.iter() {
                            if let Err(e) = stream.write(msg.as_bytes().as_slice()).await {
                                error!("connection closed: {}", e);
                                stream.shutdown().await?;
                                return Ok(());
                            }
                            stream.flush().await?;
                        }
                        continue;
                    }
                    if let Some(msg) = biz::work(&mut msg, map1, redis).await {
                        if let Err(e) = stream.write(msg.as_bytes().as_slice()).await {
                            error!("connection closed: {}", e);
                            stream.shutdown().await?;
                            return Ok(());
                        }
                        stream.flush().await?;
                        continue;
                    }
                }
            }
            msg = receiver.recv() => {
                if let Some(msg) = msg {
                    if let Err(e) = stream.write(msg.as_bytes().as_slice()).await {
                        error!("connection closed: {}", e);
                        stream.shutdown().await?;
                        return Ok(());
                    }
                    stream.flush().await?;
                }
            }
        }
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
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test() {
        tokio::spawn(async {
            println!("aaa");
        });
        let _ = tokio::time::sleep(Duration::from_secs(1));
        tokio::spawn(async {
            println!("bbb");
        });
    }
}