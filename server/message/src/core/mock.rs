use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error};
use crate::entity::msg;
use crate::util::base;

#[allow(unused)]
const BODY_BUF_LENGTH: usize = 1 << 16;

#[allow(unused)]
pub struct Client {
    address: String,
}

impl Client {
    #[allow(unused)]
    pub async fn run(address: String, sender: u64, receiver: u64) {
        tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
            let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
            let mut head_buf = &mut (*head);
            let mut body_buf = &mut (*body);
            let (s, mut r): (tokio::sync::mpsc::Sender<msg::Msg>, tokio::sync::mpsc::Receiver<msg::Msg>) = tokio::sync::mpsc::channel(10);
            let socket = &mut stream;
            tokio::spawn(async move {
                let _ = s.send(msg::Msg::ping(sender)).await;
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let _ = s.send(msg::Msg::text_str(sender, receiver, "aaa")).await;
                tokio::time::sleep(Duration::from_secs(10)).await;
            });
            loop {
                tokio::select! {
                    _n = socket.readable() => {
                        let msg = Self::read(socket, head_buf, body_buf).await.unwrap();
                        debug!("{}: {:?}", sender, msg)
                    }
                    m = r.recv() => {
                        // println!("{}: {:?}", sender, m);
                        if let Some(m) = m {
                            let _ = socket.write(m.as_bytes().as_slice()).await;
                            let _ = socket.flush().await;
                        } else {
                            continue
                        }
                    }
                }
            }
        });
    }

    #[allow(unused)]
    async fn read(stream: &mut tokio::net::TcpStream, head_buf: &mut [u8], body_buf: &mut [u8]) -> std::io::Result<msg::Msg> {
        if let Ok(readable_size) = stream.read(head_buf).await {
            if readable_size == 0 {
                debug!("connection:[{}] closed", stream.peer_addr().unwrap());
                stream.shutdown().await?;
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "connection closed"));
            }
            if readable_size != msg::HEAD_LEN {
                error!("read head error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "read head error"));
            }
            let mut head = msg::Head::from(&head_buf[..]);
            head.timestamp = base::timestamp();
            let body_length = stream.read(&mut body_buf[0..head.length as usize]).await?;
            if body_length != head.length as usize {
                error!("read body error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "read body error"));
            }
            let length = head.length;
            let msg = msg::Msg {
                head,
                payload: Vec::from(&body_buf[0..length as usize]),
            };
            Ok(msg)
        } else {
            error!("read head error");
            stream.shutdown().await?;
            Err(std::io::Error::new(std::io::ErrorKind::Other, "read head error"))
        }
    }
}