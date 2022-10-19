use crate::entity::{Head, Msg, Type, HEAD_LEN};
use crate::util::timestamp;
use byteorder::{BigEndian, ByteOrder};
use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value};
use std::fmt::{Display, Formatter};
use std::io::Read;

impl From<&[u8]> for Type {
    #[inline]
    fn from(buf: &[u8]) -> Self {
        let value = BigEndian::read_u16(&buf[4..6]);
        Self::from(value)
    }
}

impl From<u16> for Type {
    #[inline]
    fn from(value: u16) -> Self {
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

impl Into<u16> for Type {
    fn into(self) -> u16 {
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
    #[inline]
    pub(crate) fn values(&self) -> u16 {
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

impl From<&[u8]> for Head {
    #[inline]
    fn from(buf: &[u8]) -> Self {
        let extension_length = BigEndian::read_u16(&buf[0..2]);
        let payload_length = BigEndian::read_u16(&buf[2..4]);
        let typ = BigEndian::read_u16(&buf[4..6]);
        let sender = BigEndian::read_u64(&buf[6..14]);
        let receiver = BigEndian::read_u64(&buf[14..22]);
        let timestamp = BigEndian::read_u64(&buf[22..30]);
        let seq_num = BigEndian::read_u64(&buf[30..38]);
        let version = BigEndian::read_u16(&buf[38..40]);
        Self {
            payload_length,
            extension_length,
            typ: Type::from(typ),
            sender,
            receiver,
            timestamp,
            seq_num,
            version,
        }
    }
}

impl Read for Head {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() < HEAD_LEN {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "buf is too small",
            ))
        } else {
            BigEndian::write_u16(&mut buf[0..2], self.extension_length);
            BigEndian::write_u16(&mut buf[2..4], self.payload_length);
            BigEndian::write_u16(&mut buf[4..6], self.typ.values());
            BigEndian::write_u64(&mut buf[6..14], self.sender);
            BigEndian::write_u64(&mut buf[14..22], self.receiver);
            BigEndian::write_u64(&mut buf[22..30], self.timestamp);
            BigEndian::write_u64(&mut buf[30..38], self.seq_num);
            BigEndian::write_u16(&mut buf[38..40], self.version);
            Ok(HEAD_LEN)
        }
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Head [ extension_length: {}, payload_length: {}, typ: {}, sender: {}, receiver: {}, timestamp: {}, seq_num: {}, version: {} ]", self.extension_length, self.payload_length, self.typ, self.sender, self.receiver, self.timestamp, self.seq_num, self.version)
    }
}

impl From<&[u8]> for Msg {
    #[inline]
    fn from(buf: &[u8]) -> Self {
        Self(Vec::from(buf))
    }
}

impl ToRedisArgs for Msg {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg(self.as_slice());
    }
}

impl FromRedisValue for Msg {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        if let Value::Data(ref v) = *v {
            Ok(Msg::from(v.as_slice()))
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
            "Msg [ head: {}, extension: {}, payload: {} ]",
            Head::from(&self.0[0..HEAD_LEN]),
            String::from_utf8_lossy(
                &self.0[HEAD_LEN..(HEAD_LEN + self.extension_length() as usize)]
            ),
            String::from_utf8_lossy(
                &self.0[(HEAD_LEN + self.extension_length() as usize)
                    ..(HEAD_LEN + self.extension_length() as usize + self.payload_length())]
            )
        )
    }
}

impl Head {
    #[allow(unused)]
    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Msg {
    #[inline]
    pub fn read_u16(buffer: &[u8]) -> u16 {
        BigEndian::read_u16(&buffer[0..2])
    }

    #[inline]
    pub fn pre_alloc(payload_length: u16, extension_length: u16) -> Self {
        let mut buf =
            Vec::with_capacity(HEAD_LEN + payload_length as usize + extension_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN + payload_length as usize + extension_length as usize);
        }
        Self(buf)
    }

    #[inline]
    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }

    #[inline]
    pub fn extension_length(&self) -> usize {
        BigEndian::read_u16(&self.0[0..2]) as usize
    }

    #[inline]
    pub fn payload_length(&self) -> usize {
        BigEndian::read_u16(&self.0[2..4]) as usize
    }

    #[inline]
    pub fn typ(&self) -> Type {
        Type::from(BigEndian::read_u16(&self.0[4..6]))
    }

    #[inline]
    pub fn sender(&self) -> u64 {
        BigEndian::read_u64(&self.0[6..14])
    }

    #[inline]
    pub fn receiver(&self) -> u64 {
        BigEndian::read_u64(&self.0[14..22])
    }

    #[inline]
    pub fn timestamp(&self) -> u64 {
        BigEndian::read_u64(&self.0[22..30])
    }

    #[inline]
    pub fn seq_num(&self) -> u64 {
        BigEndian::read_u64(&self.0[30..38])
    }

    #[allow(unused)]
    #[inline]
    pub fn version(&self) -> u16 {
        BigEndian::read_u16(&self.0[38..40])
    }

    #[inline]
    pub fn update_extension_length(&mut self, extension_length: u16) {
        BigEndian::write_u16(&mut self.0[0..2], extension_length);
    }

    #[inline]
    pub fn update_payload_length(&mut self, payload_length: u16) {
        BigEndian::write_u16(&mut self.0[2..4], payload_length);
    }

    #[inline]
    pub fn update_type(&mut self, typ: Type) {
        BigEndian::write_u16(&mut self.0[4..6], typ.values());
    }

    #[inline]
    pub fn update_sender(&mut self, sender: u64) {
        BigEndian::write_u64(&mut self.0[6..14], sender);
    }

    #[inline]
    pub fn update_receiver(&mut self, receiver: u64) {
        BigEndian::write_u64(&mut self.0[14..22], receiver);
    }

    #[inline]
    pub fn update_timestamp(&mut self, timestamp: u64) {
        BigEndian::write_u64(&mut self.0[22..30], timestamp);
    }

    #[allow(unused)]
    #[inline]
    pub fn update_seq_num(&mut self, seq_num: u64) {
        BigEndian::write_u64(&mut self.0[30..38], seq_num);
    }

    #[allow(unused)]
    #[inline]
    pub fn update_version(&mut self, version: u16) {
        BigEndian::write_u16(&mut self.0[38..40], version);
    }

    #[allow(unused)]
    #[inline]
    pub fn extension(&self) -> &[u8] {
        let extension_length = BigEndian::read_u16(&self.as_slice()[2..4]);
        if extension_length == 0 {
            &[]
        } else {
            &self.as_slice()[HEAD_LEN..HEAD_LEN + extension_length as usize]
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn extension_mut(&mut self) -> &mut [u8] {
        let extension_length = BigEndian::read_u16(&self.as_slice()[2..4]);
        if extension_length == 0 {
            &mut []
        } else {
            &mut self.as_mut_slice()[HEAD_LEN..HEAD_LEN + extension_length as usize]
        }
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        let extension_length = BigEndian::read_u16(&self.as_slice()[0..2]);
        let payload_length = BigEndian::read_u16(&self.as_slice()[2..4]);
        if payload_length == 0 {
            &[]
        } else {
            &self.as_slice()[HEAD_LEN + extension_length as usize
                ..HEAD_LEN + extension_length as usize + payload_length as usize]
        }
    }

    #[allow(unused)]
    #[inline]
    pub(crate) fn payload_mut(&mut self) -> &mut [u8] {
        let extension_length = BigEndian::read_u16(&self.as_slice()[0..2]);
        let payload_length = BigEndian::read_u16(&self.as_slice()[2..4]);
        if payload_length == 0 {
            &mut []
        } else {
            &mut self.as_mut_slice()[HEAD_LEN + extension_length as usize
                ..HEAD_LEN + extension_length as usize + payload_length as usize]
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn ping(sender: u64) -> Self {
        let mut head = Head {
            payload_length: 4,
            extension_length: 0,
            typ: Type::Ping,
            sender,
            receiver: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(b"ping");
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn err_msg(sender: u64, receiver: u64, reason: String) -> Self {
        let mut head = Head {
            payload_length: reason.len() as u16,
            extension_length: 0,
            typ: Type::Error,
            sender,
            receiver,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(reason.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn err_msg_str(sender: u64, receiver: u64, reason: &str) -> Self {
        let mut head = Head {
            payload_length: reason.len() as u16,
            extension_length: 0,
            typ: Type::Error,
            sender,
            receiver,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(reason.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn text(sender: u64, receiver: u64, text: String) -> Self {
        let mut head = Head {
            payload_length: text.len() as u16,
            extension_length: 0,
            typ: Type::Text,
            sender,
            receiver,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(text.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn text_str(sender: u64, receiver: u64, text: &'static str) -> Self {
        let mut head = Head {
            payload_length: text.len() as u16,
            extension_length: 0,
            typ: Type::Text,
            sender,
            receiver,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(text.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn generate_ack(&self, client_timestamp: u64) -> Self {
        let time = client_timestamp.to_string();
        let mut head = Head {
            payload_length: time.len() as u16,
            extension_length: 0,
            typ: Type::Ack,
            sender: 0,
            receiver: self.sender(),
            timestamp: timestamp(),
            seq_num: self.seq_num(),
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(time.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn under_review_str(sender: u64, detail: &'static str) -> Self {
        let mut head = Head {
            payload_length: detail.len() as u16,
            extension_length: 0,
            typ: Type::UnderReview,
            sender,
            receiver: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(detail.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn internal_error() -> Self {
        let mut head = Head {
            payload_length: 0,
            extension_length: 0,
            typ: Type::InternalError,
            sender: 0,
            receiver: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn empty() -> Self {
        let mut head = Head {
            payload_length: 0,
            extension_length: 0,
            typ: Type::NA,
            sender: 0,
            receiver: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn auth(sender: u64, receiver: u64, token: String) -> Self {
        let token = token.as_bytes();
        let mut head = Head {
            payload_length: token.len() as u16,
            extension_length: 0,
            typ: Type::Auth,
            sender,
            receiver,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + head.payload_length as usize);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(token);
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn only_head(head: &mut Head) -> Self {
        let mut buf = Vec::with_capacity(HEAD_LEN);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        Self(buf)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let mut v = Vec::with_capacity(10);
        unsafe {
            v.set_len(10);
        }
        let s = v.as_mut_slice();
        s[1] = 1;
        s[2] = 2;
        println!("{:?}", v);
    }
}
