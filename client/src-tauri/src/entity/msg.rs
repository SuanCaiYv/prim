use serde::{Serialize, Deserialize};
use byteorder::ByteOrder;
use crate::util;

pub const HEAD_LEN: usize = 37;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Type {
    NA,
    // 消息部分
    Text,
    Meme,
    File,
    Image,
    Video,
    Audio,
    // 逻辑部分
    Ack,
    Box,
    Auth,
    Sync,
    Error,
    Offline,
    Heartbeat,
    UnderReview,
    InternalError,
    // 业务部分
    FriendRelationship,
    SysNotification,
}

impl From<i8> for Type {
    fn from(value: i8) -> Self {
        match value {
            1 => Type::Text,
            2 => Type::Meme,
            3 => Type::File,
            4 => Type::Image,
            5 => Type::Video,
            6 => Type::Audio,
            7 => Type::Ack,
            8 => Type::Box,
            9 => Type::Auth,
            10 => Type::Sync,
            11 => Type::Error,
            12 => Type::Offline,
            13 => Type::Heartbeat,
            14 => Type::UnderReview,
            15 => Type::InternalError,
            16 => Type::FriendRelationship,
            17 => Type::SysNotification,
            _ => Type::NA
        }
    }
}

impl Into<i8> for Type {
    fn into(self) -> i8 {
        match self {
            Type::Text => 1,
            Type::Meme => 2,
            Type::File => 3,
            Type::Image => 4,
            Type::Video => 5,
            Type::Audio => 6,
            Type::Ack => 7,
            Type::Box => 8,
            Type::Auth => 9,
            Type::Sync => 10,
            Type::Error => 11,
            Type::Offline => 12,
            Type::Heartbeat => 13,
            Type::UnderReview => 14,
            Type::InternalError => 15,
            Type::FriendRelationship => 16,
            Type::SysNotification => 17,
            _ => 0
        }
    }
}

impl Type {
    fn value(&self) -> i8 {
        match *self {
            Type::Text => 1,
            Type::Meme => 2,
            Type::File => 3,
            Type::Image => 4,
            Type::Video => 5,
            Type::Audio => 6,
            Type::Ack => 7,
            Type::Box => 8,
            Type::Auth => 9,
            Type::Sync => 10,
            Type::Error => 11,
            Type::Offline => 12,
            Type::Heartbeat => 13,
            Type::UnderReview => 14,
            Type::InternalError => 15,
            Type::FriendRelationship => 16,
            Type::SysNotification => 17,
            _ => 0
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Head {
    pub length: u16,
    pub typ: Type,
    pub sender: u64,
    pub receiver: u64,
    pub timestamp: u64,
    pub seq_num: u64,
    pub version: u16,
}

impl From<&[u8]> for Head {
    fn from(buf: &[u8]) -> Self {
        Self {
            length: byteorder::BigEndian::read_u16(&buf[0..2]),
            typ: Type::from(buf[2] as i8),
            sender: byteorder::BigEndian::read_u64(&buf[3..11]),
            receiver: byteorder::BigEndian::read_u64(&buf[11..19]),
            timestamp: byteorder::BigEndian::read_u64(&buf[19..27]),
            seq_num: byteorder::BigEndian::read_u64(&buf[27..35]),
            version: byteorder::BigEndian::read_u16(&buf[35..37]),
        }
    }
}

impl Head {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(HEAD_LEN);
        let mut arr: [u8;HEAD_LEN] = [0;HEAD_LEN];
        let mut buf = &mut arr;
        // 网络传输选择大端序，大端序符合人类阅读，小端序地位低地址，符合计算机计算
        byteorder::BigEndian::write_u16(&mut buf[0..2], self.length);
        buf[2] = self.typ.value() as u8;
        byteorder::BigEndian::write_u64(&mut buf[3..11], self.sender);
        byteorder::BigEndian::write_u64(&mut buf[11..19], self.receiver);
        byteorder::BigEndian::write_u64(&mut buf[19..27], self.timestamp);
        byteorder::BigEndian::write_u64(&mut buf[27..35], self.seq_num);
        byteorder::BigEndian::write_u16(&mut buf[35..37], self.version);
        v.extend_from_slice(buf);
        v
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Msg {
    pub head: Head,
    pub payload: Vec<u8>,
}

impl Default for Msg {
    fn default() -> Self {
        Msg {
            head: Head {
                length: 12,
                typ: Type::Text,
                sender: 1234,
                receiver: 4321,
                timestamp: 0,
                seq_num: 0,
                version: 1,
            },
            payload: Vec::from("codewithbuff"),
        }
    }
}

impl From<&[u8]> for Msg {
    fn from(buf: &[u8]) -> Self {
        Self {
            head: Head::from(buf),
            payload: Vec::from(&buf[HEAD_LEN..]),
        }
    }
}

impl From<Vec<u8>> for Msg {
    fn from(buf: Vec<u8>) -> Self {
        Self::from(buf.as_slice())
    }
}

impl From<&Vec<u8>> for Msg {
    fn from(buf: &Vec<u8>) -> Self {
        Self::from(buf.as_slice())
    }
}

impl Msg {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.head.length as usize + HEAD_LEN);
        buf.extend_from_slice(&self.head.as_bytes()[0..HEAD_LEN]);
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn ping(sender: u64) -> Self {
        Self {
            head: Head {
                length: 4,
                typ: Type::Heartbeat,
                sender,
                receiver: 0,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: Vec::from("ping"),
        }
    }

    pub fn err_msg(sender: u64, receiver: u64, reason: String) -> Self {
        Self {
            head: Head {
                length: reason.len() as u16,
                typ: Type::Error,
                sender,
                receiver,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: reason.into_bytes(),
        }
    }

    pub fn err_msg_str(sender: u64, receiver: u64, reason: &'static str) -> Self {
        Self {
            head: Head {
                length: reason.len() as u16,
                typ: Type::Error,
                sender,
                receiver,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: Vec::from(reason)
        }
    }

    pub fn text(sender: u64, receiver: u64, text: String) -> Self {
        Self {
            head: Head {
                length: text.len() as u16,
                typ: Type::Text,
                sender,
                receiver,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: text.into_bytes()
        }
    }

    pub fn text_str(sender: u64, receiver: u64, text: &'static str) -> Self {
        Self {
            head: Head {
                length: text.len() as u16,
                typ: Type::Text,
                sender,
                receiver,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: Vec::from(text)
        }
    }

    pub fn under_review(sender: u64, detail: String) -> Self {
        Self {
            head: Head {
                length: detail.len() as u16,
                typ: Type::UnderReview,
                sender,
                receiver: sender,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: detail.into_bytes()
        }
    }

    pub fn under_review_str(sender: u64, detail: &'static str) -> Self {
        Self {
            head: Head {
                length: detail.len() as u16,
                typ: Type::UnderReview,
                sender,
                receiver: sender,
                timestamp: util::base::timestamp(),
                seq_num: 0,
                version: 0
            },
            payload: Vec::from(detail)
        }
    }

    pub fn internal_error() -> Self {
        Self {
            head: Head {
                length: 0,
                typ: Type::InternalError,
                sender: 0,
                receiver: 0,
                timestamp: 0,
                seq_num: 0,
                version: 0
            },
            payload: Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::msg::Msg;

    #[test]
    fn test() {
    }
}