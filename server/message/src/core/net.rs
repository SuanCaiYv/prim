use std::time::Duration;
use ahash::AHashMap;
use redis::RedisResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn, error};
use crate::core::process;
use crate::core::process::logic::process;
use crate::entity::msg;
use crate::persistence::redis_ops;
use crate::util::base;

const BODY_BUF_LENGTH: usize = 1 << 16;
const MAX_FRIENDS_NUMBER: usize = 1 << 10;

pub type ConnectionMap = std::sync::Arc<tokio::sync::RwLock<AHashMap<u64, tokio::sync::mpsc::Sender<msg::Msg>>>>;
pub type StatusMap = std::sync::Arc<tokio::sync::RwLock<AHashMap<u64, u64>>>;
// todo 优化连接
pub type RedisOps = redis_ops::RedisOps;

pub struct Server {
    address: String,
    connection_map: ConnectionMap,
    status_map: StatusMap,
    redis_ops: RedisOps,
}

impl Server {
    pub async fn new(address_server: String, address_redis: String) -> Self {
        let redis_ops = redis_ops::RedisOps::connect(address_redis).await;
        Self {
            address: address_server,
            connection_map: std::sync::Arc::new(tokio::sync::RwLock::new(AHashMap::new())),
            status_map: std::sync::Arc::new(tokio::sync::RwLock::new(AHashMap::new())),
            redis_ops,
        }
    }

    pub async fn run(self) {
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(self.address.clone()).await.unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                (&self).handle(stream).await;
            }
        });
    }

    async fn handle(&self, mut stream: tokio::net::TcpStream) {
        let stream_address = stream.peer_addr().unwrap().to_string();
        let mut c_map = self.connection_map.clone();
        let mut s_map = self.status_map.clone();
        let mut redis_ops = self.redis_ops.clone();
        tokio::spawn(async move {
            let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
            let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
            let mut head_buf = &mut (*head);
            let mut body_buf = &mut (*body);
            let mut socket = &mut stream;

            let mut c_map_ref = &mut c_map;
            let mut s_map_ref = &mut s_map;
            let mut redis_ops_ref = &mut redis_ops;

            let (sender, mut receiver): (tokio::sync::mpsc::Sender<msg::Msg>, tokio::sync::mpsc::Receiver<msg::Msg>) = tokio::sync::mpsc::channel(1024);
            // 处理第一次读
            let mut flag = false;
            let mut receiver_id = 0;
            if let Ok(msg) = Self::read_msg_from_stream(socket, head_buf, body_buf).await {
                receiver_id = msg.head.receiver;
                if let msg::Type::Auth = msg.head.typ {
                    let auth_token = String::from_utf8_lossy(msg.payload.as_slice()).to_string();
                    let result: RedisResult<String> = redis_ops_ref.get(format!("auth-{}", msg.head.sender)).await;
                    if let Ok(auth_token_redis) = result {
                        if auth_token_redis == auth_token {
                            flag = true;
                            let mut lock = c_map_ref.write().await;
                            (*lock).insert(msg.head.sender, sender.clone());
                        } else {
                            error!("auth token error: {}", auth_token);
                        }
                    } else {
                        error!("redis read error");
                    }
                } else {
                    warn!("not auth msg");
                }
            } else {
                error!("first read failed");
            }
            if !flag {
                error!("fake connection");
                let resp = msg::Msg::err_msg_str(0, receiver_id, "fake connection");
                let _ = Self::write_msg_to_stream(socket, &resp).await;
                let _ = stream.shutdown().await;
                return;
            } else {
                info!("new connection: {}", stream_address);
                let resp = msg::Msg::pong(receiver_id);
                let _ = Self::write_msg_to_stream(socket, &resp).await;
            }
            loop {
                tokio::select! {
                    msg = Self::read_msg_from_stream(socket, head_buf, body_buf) => {
                        if let Ok(mut msg) = msg {
                            if let Ok(ref msg) = process::heartbeat::process(&mut msg, s_map_ref).await {
                                if let Err(e) = Self::write_msg_to_stream(socket, msg).await {
                                    error!("connection[{}] closed with: {}", stream_address, e);
                                    break
                                }
                            } else if let Ok(ref msg) = process::msg::process(&mut msg, c_map_ref, redis_ops_ref).await {
                                if let Err(e) = Self::write_msg_to_stream(socket, msg).await {
                                    error!("connection[{}] closed with: {}", stream_address, e);
                                    break
                                }
                            } else if let Ok(ref msg_list) = process::logic::process(&mut msg, redis_ops_ref).await {
                                let mut flag = true;
                                for msg in msg_list.into_iter() {
                                    if let Err(e) = Self::write_msg_to_stream(socket, msg).await {
                                        error!("connection[{}] closed with: {}", stream_address, e);
                                        flag = false;
                                        break;
                                    }
                                }
                                if !flag {
                                    break;
                                }
                            } else if let Ok(ref msg) = process::biz::process(&mut msg, c_map_ref, redis_ops_ref).await {
                                if let Err(e) = Self::write_msg_to_stream(socket, msg).await {
                                    error!("connection[{}] closed with: {}", stream_address, e);
                                    break
                                }
                            } else {
                                warn!("unknown msg type: {:?}", msg.head.typ);
                            }
                        } else {
                            stream.shutdown().await;
                            error!("connection [{}] closed with: {}", stream_address, "read error");
                            break;
                        }
                    }
                    msg = receiver.recv() => {
                        if let Some(ref msg) = msg {
                            if let Err(e) = Self::write_msg_to_stream(socket, msg).await {
                                error!("connection[{}] closed with: {}", socket.peer_addr().unwrap(), e);
                                break;
                            }
                        } else {
                            error!("connection [{}] closed with: {}", socket.peer_addr().unwrap(), "receiver closed");
                            break;
                        }
                    }
                }
            }
        });
    }

    async fn read_msg_from_stream(stream: &mut tokio::net::TcpStream, head_buf: &mut [u8], body_buf: &mut [u8]) -> std::io::Result<msg::Msg> {
        let readable_size = stream.read(head_buf).await?;
        if readable_size == 0 {
            error!("connection closed");
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "connection closed"));
        }
        if readable_size != msg::HEAD_LEN {
            error!("read head error");
            return Ok(msg::Msg::internal_error());
        }
        let head = msg::Head::from(&head_buf[..]);
        let body_length = stream.read(&mut body_buf[0..head.length as usize]).await?;
        if body_length != head.length as usize {
            error!("read body error");
            return Ok(msg::Msg::internal_error());
        }
        let length = head.length;
        let msg = msg::Msg {
            head,
            payload: Vec::from(&body_buf[0..length as usize]),
        };
        // debug!("read msg from {} : {:?}", stream.peer_addr().unwrap().to_string(), msg);
        Ok(msg)
    }

    async fn write_msg_to_stream(stream: &mut tokio::net::TcpStream, msg: &msg::Msg) -> std::io::Result<()> {
        // info!("write msg to {} : {:?}", stream.peer_addr().unwrap().to_string(), msg);
        stream.write(msg.as_bytes().as_slice()).await?;
        stream.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::net::{Server};

    #[tokio::test]
    async fn it_works() {
        Server::new("127.0.0.1:8190".to_string(), "127.0.0.1:6379".to_string()).await.run().await;
    }
}