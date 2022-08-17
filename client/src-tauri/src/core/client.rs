use std::io::{Read, Write};
use std::str::FromStr;
use crate::entity::msg;
use crate::util;

const BODY_BUF_LENGTH: usize = 1 << 16;
// 4096，所以间隔是4000ms
const ACK_ARRAY_LENGTH: usize = 1 << 16;
const MOD_VALUE: u64 = (ACK_ARRAY_LENGTH as u64) - 1;

const ACK_ARRAY: [bool;ACK_ARRAY_LENGTH] = [false;ACK_ARRAY_LENGTH];

pub type Sender = std::sync::mpsc::SyncSender<msg::Msg>;
pub type Receiver = std::sync::mpsc::Receiver<msg::Msg>;
type Stream = std::net::TcpStream;

pub struct Client {
    // 内部从这里写
    read_sender: Sender,
    // 内部从这里读
    write_receiver: Option<Receiver>,
    // 外界从这里写
    write_sender: Sender,
    // 外界从这里读
    read_receiver: Option<Receiver>,
    // 仅作关闭使用，因为读写都会被占用
    close_stream: Stream,
    timer: delay_timer::prelude::DelayTimer,
}

impl Client {
    pub fn connect(address: String) -> std::io::Result<Self> {
        let mut timer = delay_timer::prelude::DelayTimer::new();
        for i in 0..5 {
            let mut stream = std::net::TcpStream::connect(address.clone());
            if let Ok(close_stream) = stream {
                let (read_sender, read_receiver) = std::sync::mpsc::sync_channel(1024);
                let (write_sender, write_receiver): (Sender, Receiver) = std::sync::mpsc::sync_channel(1024);
                return Ok(Self {
                    read_sender,
                    read_receiver: Some(read_receiver),
                    write_sender,
                    write_receiver: Some(write_receiver),
                    close_stream,
                    timer,
                });
            }
            std::thread::sleep(std::time::Duration::from_secs(1))
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, "connect failed"))
    }

    // 因为只能获取write_stream的锁，所以同时关闭读写来实现关闭Socket的操作
    pub fn close(&mut self) {
        let _ = self.close_stream.shutdown(std::net::Shutdown::Both);
    }

    pub fn write(&mut self, msg: msg::Msg) -> std::io::Result<()> {
        if let Err(_) = self.write_sender.send(msg) {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "send failed"));
        } else {
            Ok(())
        }
    }

    // 此方法只能调用一次
    pub fn read(&mut self) -> Receiver {
        self.read_receiver.take().unwrap()
    }

    // 此方法必须在连接之后调用，否则无法运行
    pub fn run(&mut self) {
        let mut read_stream = self.close_stream.try_clone().unwrap();
        let mut write_stream = self.close_stream.try_clone().unwrap();
        let mut read_sender1 = self.read_sender.clone();
        let mut read_sender2 = self.read_sender.clone();
        let mut write_receiver = self.write_receiver.take().unwrap();
        let mut timer = self.timer.clone();
        // 处理读
        std::thread::spawn(move || {
            let mut head: Box<[u8; msg::HEAD_LEN]> = Box::new([0; msg::HEAD_LEN]);
            let mut body: Box<[u8; BODY_BUF_LENGTH]> = Box::new([0; BODY_BUF_LENGTH]);
            let mut head_buf = &mut (*head);
            let mut body_buf = &mut (*body);
            let mut stream = &mut read_stream;
            let mut sender = &mut read_sender1;
            loop {
                if let Ok(msg) = Self::read_msg(stream, head_buf, body_buf) {
                    match msg.head.typ {
                        msg::Type::Ack => {
                            let index = u64::from_str(&String::from_utf8_lossy(msg.payload.as_slice())).unwrap() as usize;
                            ACK_ARRAY[index] = false;
                        },
                        msg::Type::Offline => {
                            if let Err(_) = sender.send(msg::Msg::under_review_str(msg.head.sender, "FORCE_OFFLINE")) {
                                break;
                            }
                        },
                        msg::Type::Error => {
                            if let Err(_) = sender.send(msg::Msg::under_review(msg.head.sender, String::from_utf8_lossy(msg.payload.as_slice()).to_string())) {
                                break;
                            }
                        },
                        _ => {
                            if let Err(_) = sender.send(msg) {
                                break;
                            }
                        }
                    }
                } else {
                    return;
                }
            };
        });
        // 处理写
        std::thread::spawn(move || {
            let mut receiver = &mut write_receiver;
            let mut sender = &mut read_sender2;
            let mut stream = &mut write_stream;
            let mut timer = &mut timer;
            loop {
                if let Ok(mut msg) = receiver.recv() {
                    msg.head.timestamp = util::base::timestamp();
                    // 等价于index = timestamp % ACK_ARRAY_LENGTH
                    let index = (msg.head.timestamp & (MOD_VALUE) as u64) as usize;
                    ACK_ARRAY[index] = true;
                    let mut sender = sender.clone();
                    let task = delay_timer::prelude::TaskBuilder::default()
                        .set_task_id(msg.head.timestamp)
                        .set_frequency_once_by_seconds(3)
                        .spawn_routine(move || {
                            if ACK_ARRAY[index] {
                                sender.send(msg::Msg::err_msg_str(msg.head.sender, msg.head.sender, "SEND_MSG_TIMEOUT")).unwrap();
                            }
                        });
                    if let Err(_) = stream.write(msg.as_bytes().as_slice()) {
                        return;
                    };
                    if let Err(_) = stream.flush() {
                        return;
                    };
                } else {
                    return;
                }
            }
        });
    }

    fn read_msg(stream: &mut std::net::TcpStream, head_buf: &mut [u8], body_buf: &mut [u8]) -> std::io::Result<msg::Msg> {
        let readable_size = stream.read(head_buf)?;
        if readable_size == 0 {
            println!("connection closed");
            stream.shutdown(std::net::Shutdown::Both)?;
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "connection closed"));
        }
        if readable_size != msg::HEAD_LEN {
            println!("read head error");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "read head error"));
        }
        let mut head = msg::Head::from(&head_buf[..]);
        let body_length = stream.read(&mut body_buf[0..head.length as usize])?;
        if body_length != head.length as usize {
            println!("read body error");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "read body error"));
        }
        let length = head.length;
        let msg = msg::Msg {
            head,
            payload: Vec::from(&body_buf[0..length as usize]),
        };
        Ok(msg)
    }

    // 此方法应该只被调用一次
    pub fn heartbeat(&mut self, sender_id: u64) {
        let sender = self.write_sender.clone();
        let task = delay_timer::prelude::TaskBuilder::default()
            .set_task_id(util::base::timestamp())
            .set_frequency_repeated_by_seconds(3)
            .spawn_routine(move || {
                if let Err(_) = sender.send(msg::Msg::ping(sender_id, 0)) {
                    return;
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
        let mut client = super::Client::connect("127.0.0.1:8190".to_string()).unwrap();
        client.run();
        let mut msg_receiver = client.read();
        let not_use = client.heartbeat(1);
        std::thread::sleep(std::time::Duration::from_millis(3100));
        client.write(msg::Msg::text_str(1, 0, "aaa")).unwrap();
        println!("{:?}", msg_receiver.recv().unwrap());
        client.close();
    }
}