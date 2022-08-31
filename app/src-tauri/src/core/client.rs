use std::future::Future;
use std::str::FromStr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, info, warn};

use crate::entity::msg;
use crate::util;

const BODY_BUF_LENGTH: usize = 1 << 16;
// 4096，所以间隔是4000ms
const ACK_ARRAY_LENGTH: usize = 1 << 16;
const MOD_VALUE: u64 = (ACK_ARRAY_LENGTH as u64) - 1;

const ACK_ARRAY: [bool;ACK_ARRAY_LENGTH] = [false;ACK_ARRAY_LENGTH];

pub type Sender = tokio::sync::mpsc::Sender<msg::Msg>;
pub type Receiver = tokio::sync::mpsc::Receiver<msg::Msg>;
type TokioSyncResult<T, E> = Result<T, tokio::sync::mpsc::error::SendError<E>>;
pub type IoResult<T> = std::io::Result<T>;

pub struct Client {
    // 内部从这里写
    read_sender: Sender,
    // 内部从这里读
    write_receiver: Option<Receiver>,
    // 外界从这里写
    write_sender: Sender,
    // 外界从这里读
    read_receiver: Option<Receiver>,
    close_sender: tokio::sync::mpsc::Sender<()>,
    close_receiver: Option<tokio::sync::mpsc::Receiver<()>>,
    stream: Option<tokio::net::TcpStream>,
    timer: delay_timer::prelude::DelayTimer,
}

impl Client {
    pub async fn connect(address: String) -> IoResult<Self> {
        let timer = delay_timer::prelude::DelayTimer::new();
        for _i in 0..5 {
            let stream = tokio::net::TcpStream::connect(address.clone()).await;
            if let Ok(stream) = stream {
                let (close_sender, close_receiver) = tokio::sync::mpsc::channel(1);
                let (read_sender, read_receiver) = tokio::sync::mpsc::channel(1024);
                let (write_sender, write_receiver) = tokio::sync::mpsc::channel(1024);
                return Ok(Self {
                    read_sender,
                    read_receiver: Some(read_receiver),
                    write_sender,
                    write_receiver: Some(write_receiver),
                    stream: Some(stream),
                    close_sender,
                    close_receiver: Some(close_receiver),
                    timer,
                });
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        error!("can not connect to {}", address);
        Err(std::io::Error::new(std::io::ErrorKind::Other, "connect failed"))
    }

    pub async fn close(&self) {
        let _ = self.timer.stop_delay_timer();
        let _ = self.close_sender.send(()).await;
        debug!("closed");
    }

    pub async fn refresh(&self) {
        let _ = self.timer.stop_delay_timer();
    }

    pub fn data_in(&self) -> Sender {
        self.write_sender.clone()
    }

    pub fn data_out(&mut self) -> Receiver {
        self.read_receiver.take().unwrap()
    }

    // 此方法必须在连接之后调用，否则无法运行
    pub fn run(&mut self) {
        let mut stream = self.stream.take().unwrap();
        let mut read_sender = self.read_sender.clone();
        let mut write_receiver = self.write_receiver.take().unwrap();
        let mut close_receiver = self.close_receiver.take().unwrap();
        let mut timer = self.timer.clone();

        tokio::spawn(async move {
            let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
            let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
            let mut head_buf = &mut (*head);
            let mut body_buf = &mut (*body);

            let (mut reader, mut writer) = stream.split();
            let mut reader = &mut reader;
            let mut writer = &mut writer;
            let write_receiver = &mut write_receiver;
            let read_sender = &mut read_sender;
            let close_receiver = &mut close_receiver;
            let timer = &mut timer;
            loop {
                tokio::select! {
                    msg = Self::read_msg(reader, head_buf, body_buf) => {
                        if let Ok(msg) = msg {
                            if let Err(_) = Self::deal_read(msg, read_sender).await {
                                error!("send msg error");
                                break;
                            }
                        } else {
                            break;
                        }
                    },
                    msg = write_receiver.recv() => {
                        if let Some(msg) = msg {
                            debug!("send msg {:?}", msg);
                            if let Err(_) = Self::deal_write(&msg, writer, read_sender, timer).await {
                                error!("write data error");
                                break;
                            }
                        } else {
                            error!("write_receiver closed");
                            break;
                        }
                    },
                    _ = close_receiver.recv() => {
                        break;
                    }
                }
            }
            if let Err(err) = stream.shutdown().await {
                error!("{:?}", err);
            }
            let _ = timer.stop_delay_timer();
            info!("already shutdown connect");
        });
    }

    async fn read_msg<'a>(stream: &mut tokio::net::tcp::ReadHalf<'a>, head_buf: &mut [u8], body_buf: &mut [u8]) -> IoResult<msg::Msg> {
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
        Ok(msg)
    }

    async fn write_msg<'a>(stream: &mut tokio::net::tcp::WriteHalf<'a>, msg: &msg::Msg) -> IoResult<()> {
        let _ = stream.write(msg.as_bytes().as_slice()).await?;
        let _ = stream.flush().await?;
        Ok(())
    }

    async fn deal_read(msg: msg::Msg, read_sender: &mut Sender) -> TokioSyncResult<(), msg::Msg> {
        match msg.head.typ {
            msg::Type::Ack => {
                let timestamp = u64::from_str(&String::from_utf8_lossy(msg.payload.as_slice())).unwrap();
                let index = (timestamp & MOD_VALUE) as usize;
                ACK_ARRAY[index] = false;
                read_sender.send(msg).await?
            },
            msg::Type::Offline => {
                read_sender.send(msg::Msg::under_review_str(msg.head.sender, "FORCE_OFFLINE")).await?
            },
            msg::Type::Error => {
                read_sender.send(msg::Msg::under_review(msg.head.sender, String::from_utf8_lossy(msg.payload.as_slice()).to_string())).await?
            },
            msg::Type::Heartbeat => {
            },
            _ => {
                read_sender.send(msg).await?
            }
        };
        Ok(())
    }

    async fn deal_write<'a>(msg: &msg::Msg, writer: &mut tokio::net::tcp::WriteHalf<'a>, read_sender: &mut Sender, timer: &mut delay_timer::prelude::DelayTimer) -> IoResult<()> {
        match msg.head.typ {
            msg::Type::Text | msg::Type::Meme | msg::Type::File | msg::Type::Image | msg::Type::Audio | msg::Type::Video => {
                // 等价于index = timestamp % ACK_ARRAY_LENGTH
                let index = (msg.head.timestamp & MOD_VALUE) as usize;
                ACK_ARRAY[index] = true;

                let read_sender = read_sender.clone();
                let sender_id = msg.head.sender;
                let msg_identifier = format!("SEND_MSG_TIMEOUT-{}", util::base::who_we_are(msg.head.sender, msg.head.receiver));
                let timestamp = msg.head.timestamp;
                let body = move || {
                    let identifier = msg_identifier.clone();
                    let read_sender = read_sender.clone();
                    async move {
                        if ACK_ARRAY[index] {
                            let mut msg = msg::Msg::err_msg(sender_id, sender_id, identifier);
                            msg.head.timestamp = timestamp;
                            let _ = read_sender.send(msg).await.unwrap();
                        }
                    }
                };
                let task = delay_timer::prelude::TaskBuilder::default()
                    .set_task_id(msg.head.timestamp)
                    .set_frequency_once_by_seconds(3)
                    .spawn_async_routine(body);
                timer.insert_task(task.unwrap()).unwrap();
            },
            _ => {}
        };
        if let Err(_) = Self::write_msg(writer, msg).await {
            Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "can't write data"))
        } else {
            return Ok(());
        }
    }

    pub fn heartbeat(&self, sender_id: u64) {
        let sender = self.data_in();
        let task = delay_timer::prelude::TaskBuilder::default()
            .set_task_id(util::base::timestamp())
            .set_frequency_repeated_by_seconds(3)
            .spawn_async_routine(move || {
                // todo 几把定时器有问题，早晚给换了
                let sender = sender.clone();
                async move {
                    if let Err(_) = sender.send(msg::Msg::ping(sender_id)).await {
                        error!("send heartbeat error");
                    }
                }
            });
        self.timer.add_task(task.unwrap()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::msg;

    #[test]
    fn test() {
    }
}