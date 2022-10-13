use std::fmt::{Display, Formatter};

use byteorder::ByteOrder;
use redis::*;
use serde::{Deserialize, Serialize};

use crate::util::timestamp;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
    Auth,
    Ping,
    Echo,
    Error,
    Offline,
    UnderReview,
    InternalError,
    // 业务部分
    SysNotification,
    FriendRelationship,
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
            8 => Type::Auth,
            9 => Type::Ping,
            10 => Type::Echo,
            11 => Type::Error,
            12 => Type::Offline,
            13 => Type::UnderReview,
            14 => Type::InternalError,
            15 => Type::SysNotification,
            16 => Type::FriendRelationship,
            _ => Type::NA,
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
            Type::Auth => 8,
            Type::Ping => 9,
            Type::Echo => 10,
            Type::Error => 11,
            Type::Offline => 12,
            Type::UnderReview => 13,
            Type::InternalError => 14,
            Type::SysNotification => 15,
            Type::FriendRelationship => 16,
            _ => 0,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::Text => "Text",
                Type::Meme => "Meme",
                Type::File => "File",
                Type::Image => "Image",
                Type::Video => "Video",
                Type::Audio => "Audio",
                Type::Ack => "Ack",
                Type::Auth => "Auth",
                Type::Ping => "Ping",
                Type::Echo => "Echo",
                Type::Error => "Error",
                Type::Offline => "Offline",
                Type::UnderReview => "UnderReview",
                Type::InternalError => "InternalError",
                Type::SysNotification => "SysNotification",
                Type::FriendRelationship => "FriendRelationship",
                _ => "NA",
            }
        )
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
            Type::Auth => 8,
            Type::Ping => 9,
            Type::Echo => 10,
            Type::Error => 11,
            Type::Offline => 12,
            Type::UnderReview => 13,
            Type::InternalError => 14,
            Type::SysNotification => 15,
            Type::FriendRelationship => 16,
            _ => 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Head {
    pub length: u16,
    pub typ: Type,
    // 作为消息类型指出发送者和接收者
    // 作为其他类型可能会指出此次消息属于的双端
    pub sender: u64,
    pub receiver: u64,
    pub timestamp: u64,
    pub seq_num: u64,
    // [0, 1 << 8)属于消息使用，[1 << 8, 1 << 16)属于逻辑使用
    pub version: u16,
    pub extension: u64,
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
            extension: byteorder::BigEndian::read_u64(&buf[37..45]),
        }
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Head [ length: {}, typ: {}, sender: {}, receiver: {}, timestamp: {}, seq_num: {}, version: {} ]", self.length, self.typ, self.sender, self.receiver, self.timestamp, self.seq_num, self.version)
    }
}

impl Head {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(super::HEAD_LEN);
        let mut arr: [u8; super::HEAD_LEN] = [0; super::HEAD_LEN];
        let buf = &mut arr;
        // 网络传输选择大端序，大端序符合人类阅读，小端序地位低地址，符合计算机计算
        byteorder::BigEndian::write_u16(&mut buf[0..2], self.length);
        buf[2] = self.typ.value() as u8;
        byteorder::BigEndian::write_u64(&mut buf[3..11], self.sender);
        byteorder::BigEndian::write_u64(&mut buf[11..19], self.receiver);
        byteorder::BigEndian::write_u64(&mut buf[19..27], self.timestamp);
        byteorder::BigEndian::write_u64(&mut buf[27..35], self.seq_num);
        byteorder::BigEndian::write_u16(&mut buf[35..37], self.version);
        byteorder::BigEndian::write_u64(&mut buf[37..45], self.extension);
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
                extension: 0,
            },
            payload: Vec::from("codewithbuff"),
        }
    }
}

impl From<&[u8]> for Msg {
    fn from(buf: &[u8]) -> Self {
        Self {
            head: Head::from(buf),
            payload: Vec::from(&buf[super::HEAD_LEN..]),
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

impl ToRedisArgs for Msg {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg(serde_json::to_vec(self).unwrap().as_slice());
    }
}

impl FromRedisValue for Msg {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        if let Value::Data(ref v) = *v {
            let result: serde_json::Result<Msg> = serde_json::from_slice(v.as_slice());
            if let Err(_) = result {
                return Err(RedisError::from((
                    ErrorKind::TypeError,
                    "deserialize failed",
                )));
            } else {
                Ok(result.unwrap())
            }
        } else {
            Err(RedisError::from((
                ErrorKind::TypeError,
                "deserialize failed",
            )))
        }
    }
}

impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Msg [ head: {}, payload: {} ]",
            self.head,
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl Msg {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.head.length as usize + super::HEAD_LEN);
        buf.extend_from_slice(&self.head.as_bytes()[0..super::HEAD_LEN]);
        buf.extend_from_slice(&self.payload);
        buf
    }

    #[allow(unused)]
    pub fn duplicate(&self) -> Self {
        Self {
            head: self.head.clone(),
            payload: self.payload.clone(),
        }
    }

    #[allow(unused)]
    pub fn ping(sender: u64) -> Self {
        Self {
            head: Head {
                length: 4,
                typ: Type::Ping,
                sender,
                receiver: 0,
                timestamp: crate::util::timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::from("ping"),
        }
    }

    #[allow(unused)]
    pub fn err_msg(sender: u64, receiver: u64, reason: String) -> Self {
        Self {
            head: Head {
                length: reason.len() as u16,
                typ: Type::Error,
                sender,
                receiver,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: reason.into_bytes(),
        }
    }

    #[allow(unused)]
    pub fn err_msg_str(sender: u64, receiver: u64, reason: &str) -> Self {
        Self {
            head: Head {
                length: reason.len() as u16,
                typ: Type::Error,
                sender,
                receiver,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::from(reason),
        }
    }

    #[allow(unused)]
    pub fn text(sender: u64, receiver: u64, text: String) -> Self {
        Self {
            head: Head {
                length: text.len() as u16,
                typ: Type::Text,
                sender,
                receiver,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: text.into_bytes(),
        }
    }

    #[allow(unused)]
    pub fn text_str(sender: u64, receiver: u64, text: &'static str) -> Self {
        Self {
            head: Head {
                length: text.len() as u16,
                typ: Type::Text,
                sender,
                receiver,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::from(text),
        }
    }

    #[allow(unused)]
    pub fn generate_ack(&self, client_timestamp: u64) -> Self {
        let time = client_timestamp.to_string();
        Self {
            head: Head {
                length: time.len() as u16,
                typ: Type::Ack,
                sender: 0,
                receiver: self.head.sender,
                timestamp: timestamp(),
                seq_num: self.head.seq_num,
                version: 0,
                extension: 0,
            },
            // todo
            payload: Vec::from(time),
        }
    }

    #[allow(unused)]
    pub fn under_review_str(sender: u64, detail: &'static str) -> Self {
        Self {
            head: Head {
                length: detail.len() as u16,
                typ: Type::UnderReview,
                sender,
                receiver: sender,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::from(detail),
        }
    }

    #[allow(unused)]
    pub fn internal_error() -> Self {
        Self {
            head: Head {
                length: 0,
                typ: Type::InternalError,
                sender: 0,
                receiver: 0,
                timestamp: 0,
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn empty() -> Self {
        Self {
            head: Head {
                length: 0,
                typ: Type::NA,
                sender: 0,
                receiver: 0,
                timestamp: 0,
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn auth(sender: u64, receiver: u64, token: String) -> Self {
        Self {
            head: Head {
                length: token.len() as u16,
                typ: Type::Auth,
                sender,
                receiver,
                timestamp: timestamp(),
                seq_num: 0,
                version: 0,
                extension: 0,
            },
            payload: Vec::from(token),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}
