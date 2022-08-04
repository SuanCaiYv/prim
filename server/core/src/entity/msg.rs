use byteorder::ByteOrder;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Msg<'a> {
    pub head: Head,
    pub payload: &'a [u8],
}

pub const HEAD_LEN: usize = 37;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Head {
    pub length: u16,
    pub typ: i8,
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
            typ: buf[2] as i8,
            sender: byteorder::BigEndian::read_u64(&buf[3..11]),
            receiver: byteorder::BigEndian::read_u64(&buf[11..19]),
            timestamp: byteorder::BigEndian::read_u64(&buf[19..27]),
            seq_num: byteorder::BigEndian::read_u64(&buf[27..35]),
            version: byteorder::BigEndian::read_u16(&buf[35..37]),
        }
    }
}

impl Head {
    pub fn as_bytes(&self) -> Box<[u8]> {
        let mut array: [u8;HEAD_LEN] = [0;HEAD_LEN];
        let mut buf = &mut array[..];
        // 网络传输选择大端序，大端序符合人类阅读，小端序地位低地址，符合计算机计算
        byteorder::BigEndian::write_u16(&mut buf[0..2], self.length);
        buf[2] = self.typ as u8;
        byteorder::BigEndian::write_u64(&mut buf[3..11], self.sender);
        byteorder::BigEndian::write_u64(&mut buf[11..19], self.receiver);
        byteorder::BigEndian::write_u64(&mut buf[19..27], self.timestamp);
        byteorder::BigEndian::write_u64(&mut buf[27..35], self.seq_num);
        byteorder::BigEndian::write_u16(&mut buf[35..37], self.version);
        Box::new(array)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Type {
    NA(i8),
    // 消息部分
    Text(i8),
    Meme(i8),
    Image(i8),
    Video(i8),
    Audio(i8),
    File(i8),
    // 逻辑部分
    Ack(i8),
    Sync(i8),
    Offline(i8),
    Heartbeat(i8)
}

impl Type {
    pub fn from_i16(value: i8) -> Self {
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

    pub fn value(&self) -> i8 {
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

impl<'a> Msg<'a> {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.head.length as usize + HEAD_LEN);
        buf.extend_from_slice(&self.head.as_bytes()[0..HEAD_LEN]);
        buf.extend_from_slice(&self.payload);
        buf
    }
}

impl<'a> From<&'a [u8]> for Msg<'a> {
    fn from(buf: &'a [u8]) -> Self {
        Self {
            head: Head::from(buf),
            payload: &buf[HEAD_LEN..],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Msg;

    #[test]
    fn test() {
        let msg = Msg::default();
        println!("{:?}", msg);
        let bytes = msg.as_bytes();
        let buf = bytes.as_slice();
        let msg1 = Msg::from(buf);
        println!("{:?}", msg1);
    }
}