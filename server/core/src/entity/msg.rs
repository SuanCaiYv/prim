use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Msg<'a> {
    pub head: Head,
    pub payload: &'a [u8],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Head {
    pub length: u16,
    pub typ: i16,
    pub sender: u64,
    pub receiver: u64,
    pub timestamp: u64,
    pub seq_num: u64,
    pub version: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Type {
    NA(i16),
    // 消息部分
    Text(i16),
    Meme(i16),
    Image(i16),
    Video(i16),
    Audio(i16),
    File(i16),
    // 逻辑部分
    Ack(i16),
    Sync(i16),
    Offline(i16),
    Heartbeat(i16)
}

impl Type {
    pub fn from_i16(value: i16) -> Self {
        match value {
            1 => Type::Text(1),
            2 => Type::Meme(2),
            3 => Type::Image(3),
            4 => Type::Video(4),
            5 => Type::Audio(5),
            6 => Type::File(6),
            7 => Type::Ack(7),
            8 => Type::Sync(8),
            9 => Type::Offline(9),
            10 => Type::Heartbeat(10),
            _ => Type::NA(0)
        }
    }

    pub fn value(&self) -> i16 {
        match *self {
            Type::Text(val) => val,
            Type::Meme(val) => val,
            Type::Image(val) => val,
            Type::Video(val) => val,
            Type::Audio(val) => val,
            Type::File(val) => val,
            Type::Ack(val) => val,
            Type::Sync(val) => val,
            Type::Offline(val) => val,
            Type::Heartbeat(val) => val,
            _ => 0
        }
    }
}

impl Default for Msg<'static> {
    fn default() -> Self {
        Msg {
            head: Head {
                length: 12,
                typ: 1,
                sender: 1234,
                receiver: 4321,
                timestamp: 0,
                seq_num: 0,
                version: 1,
            },
            payload: "hello world!".as_bytes(),
        }
    }
}