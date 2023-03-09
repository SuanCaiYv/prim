use std::{
    fmt::{Display, Formatter},
    io::Read,
};

use byteorder::{BigEndian, ByteOrder};
use num_traits::FromPrimitive;
use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value};
use rusqlite::{types::ToSqlOutput, ToSql};

use crate::util::timestamp;

use super::{Head, InnerHead, Msg, Type, HEAD_LEN};

pub(self) const BIT_MASK_LEFT_46: u64 = 0xFFFF_C000_0000_0000;
pub(self) const BIT_MASK_RIGHT_46: u64 = 0x0000_3FFF_FFFF_FFFF;
pub(self) const BIT_MASK_LEFT_50: u64 = 0xFFFC_0000_0000_0000;
pub(self) const BIT_MASK_RIGHT_50: u64 = 0x0003_FFFF_FFFF_FFFF;
pub(self) const BIT_MASK_LEFT_12: u64 = 0xFFF0_0000_0000_0000;
pub(self) const BIT_MASK_RIGHT_12: u64 = 0x000F_FFFF_FFFF_FFFF;

impl From<u16> for Type {
    #[inline]
    fn from(value: u16) -> Self {
        let e: Option<Type> = FromPrimitive::from_u16(value);
        match e {
            Some(e) => e,
            None => Type::NA,
        }
    }
}

impl From<i16> for Type {
    #[inline]
    fn from(value: i16) -> Self {
        Self::from(value as u16)
    }
}

impl Into<u16> for Type {
    fn into(self) -> u16 {
        self as u16
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::NA
    }
}

// impl<'a> sqlx::FromRow<'a, PgRow> for Type {
//     fn from_row(row: &'a PgRow) -> Result<Self, sqlx::Error> {
//         Ok(Type::from(row.try_get::<i16, _>("type")? as u16))
//     }
// }

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::Ack => "Ack",
                Type::Text => "Text",
                Type::Meme => "Meme",
                Type::File => "File",
                Type::Image => "Image",
                Type::Video => "Video",
                Type::Audio => "Audio",
                Type::Edit => "Edit",
                Type::Withdraw => "Withdraw",
                Type::Auth => "Auth",
                Type::Ping => "Ping",
                Type::Echo => "Echo",
                Type::Error => "Error",
                Type::BeOffline => "Offline",
                Type::InternalError => "InternalError",
                Type::SystemMessage => "SysNotification",
                Type::AddFriend => "AddFriend",
                Type::RemoveFriend => "RemoveFriend",
                Type::JoinGroup => "JoinGroup",
                Type::LeaveGroup => "LeaveGroup",
                Type::Noop => "Noop",
                Type::InterruptSignal => "InterruptSignal",
                Type::UserNodeMapChange => "UserNodeMapChange",
                Type::MessageNodeRegister => "NodeRegister",
                Type::MessageNodeUnregister => "NodeUnregister",
                Type::RecorderNodeRegister => "RecorderNodeRegister",
                Type::RecorderNodeUnregister => "RecorderNodeUnregister",
                Type::SchedulerNodeRegister => "SchedulerNodeRegister",
                Type::SchedulerNodeUnregister => "SchedulerNodeUnregister",
                _ => "NA",
            }
        )
    }
}

impl Type {
    #[inline]
    pub fn value(&self) -> u16 {
        *self as u16
    }
}

impl ToSql for Type {
    fn to_sql(&self) -> std::result::Result<ToSqlOutput, rusqlite::Error> {
        let to_sql = ToSqlOutput::from(*self as u16);
        Ok(to_sql)
    }
}

impl From<&[u8]> for Head {
    #[inline]
    fn from(buf: &[u8]) -> Self {
        let version_with_sender = BigEndian::read_u64(&buf[0..8]);
        let node_id_with_receiver = BigEndian::read_u64(&buf[8..16]);
        let type_with_extension_length_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        let payload_length_with_seq_num = BigEndian::read_u64(&buf[24..32]);
        Self {
            version_with_sender,
            node_id_with_receiver,
            type_with_extension_length_with_timestamp,
            payload_length_with_seq_num,
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
            BigEndian::write_u64(&mut buf[0..8], self.version_with_sender);
            BigEndian::write_u64(&mut buf[8..16], self.node_id_with_receiver);
            BigEndian::write_u64(
                &mut buf[16..24],
                self.type_with_extension_length_with_timestamp,
            );
            BigEndian::write_u64(&mut buf[24..32], self.payload_length_with_seq_num);
            Ok(HEAD_LEN)
        }
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let head = InnerHead::from(self);
        write!(f, "Head [ extension_length: {}, payload_length: {}, typ: {}, sender: {}, receiver: {}, node_id: {}, timestamp: {}, seq_num: {}, version: {} ]", head.extension_length, head.payload_length, head.typ, head.sender, head.receiver, head.node_id, head.timestamp, head.seq_num, head.version)
    }
}

impl Head {
    #[inline]
    pub(crate) fn extension_length(buf: &[u8]) -> usize {
        let type_with_extension_length_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        ((type_with_extension_length_with_timestamp & BIT_MASK_RIGHT_12) >> 46) as usize
    }

    #[inline]
    pub(crate) fn payload_length(buf: &[u8]) -> usize {
        let payload_length_with_seq_num = BigEndian::read_u64(&buf[24..32]);
        (payload_length_with_seq_num >> 50) as usize
    }

    #[inline]
    pub(crate) fn typ(buf: &[u8]) -> Type {
        let type_extension_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        Type::from((type_extension_with_timestamp >> 52) as u16)
    }

    #[inline]
    pub(crate) fn sender(buf: &[u8]) -> u64 {
        let version_with_sender = BigEndian::read_u64(&buf[0..8]);
        (version_with_sender & BIT_MASK_RIGHT_46) as u64
    }

    #[inline]
    pub(crate) fn receiver(buf: &[u8]) -> u64 {
        let node_id_with_receiver = BigEndian::read_u64(&buf[8..16]);
        (node_id_with_receiver & BIT_MASK_RIGHT_46) as u64
    }

    #[inline]
    pub(crate) fn node_id(buf: &[u8]) -> u32 {
        let node_id_with_receiver = BigEndian::read_u64(&buf[8..16]);
        (node_id_with_receiver >> 46) as u32
    }

    #[inline]
    pub(crate) fn timestamp(buf: &[u8]) -> u64 {
        let type_extension_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        (type_extension_with_timestamp & BIT_MASK_RIGHT_46) as u64
    }

    #[inline]
    pub(crate) fn seq_num(buf: &[u8]) -> u64 {
        let payload_length_with_seq_num = BigEndian::read_u64(&buf[24..32]);
        (payload_length_with_seq_num & BIT_MASK_RIGHT_50) as u64
    }

    #[inline]
    pub(crate) fn version(buf: &[u8]) -> u32 {
        let version_with_sender = BigEndian::read_u64(&buf[0..8]);
        (version_with_sender >> 46) as u32
    }

    #[inline]
    pub(crate) fn set_version(buf: &mut [u8], version: u32) {
        let version_with_sender = BigEndian::read_u64(&buf[0..8]);
        let version_with_sender =
            (version_with_sender & BIT_MASK_RIGHT_46) | ((version as u64) << 46);
        BigEndian::write_u64(&mut buf[0..8], version_with_sender);
    }

    #[inline]
    pub(crate) fn set_sender(buf: &mut [u8], sender: u64) {
        let version_with_sender = BigEndian::read_u64(&buf[0..8]);
        let version_with_sender =
            (version_with_sender & BIT_MASK_LEFT_46) | (sender & BIT_MASK_RIGHT_46);
        BigEndian::write_u64(&mut buf[0..8], version_with_sender);
    }

    #[inline]
    pub(crate) fn set_receiver(buf: &mut [u8], receiver: u64) {
        let node_id_with_receiver = BigEndian::read_u64(&buf[8..16]);
        let node_id_with_receiver =
            (node_id_with_receiver & BIT_MASK_LEFT_46) | (receiver & BIT_MASK_RIGHT_46);
        BigEndian::write_u64(&mut buf[8..16], node_id_with_receiver);
    }

    #[inline]
    pub(crate) fn set_node_id(buf: &mut [u8], node_id: u32) {
        let node_id_with_receiver = BigEndian::read_u64(&buf[8..16]);
        let node_id_with_receiver =
            (node_id_with_receiver & BIT_MASK_RIGHT_46) | ((node_id as u64) << 46);
        BigEndian::write_u64(&mut buf[8..16], node_id_with_receiver);
    }

    #[inline]
    pub(crate) fn set_type(buf: &mut [u8], typ: Type) {
        let type_extension_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        let type_extension_with_timestamp =
            (type_extension_with_timestamp & BIT_MASK_RIGHT_12) | ((typ.value() as u64) << 52);
        BigEndian::write_u64(&mut buf[16..24], type_extension_with_timestamp);
    }

    #[inline]
    pub(crate) fn set_extension_length(buf: &mut [u8], extension_length: usize) {
        let type_extension_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        let type_extension_with_timestamp = (type_extension_with_timestamp & BIT_MASK_LEFT_12)
            | (((extension_length as u64) << 46)
                | (type_extension_with_timestamp & BIT_MASK_RIGHT_46));
        BigEndian::write_u64(&mut buf[16..24], type_extension_with_timestamp);
    }

    #[inline]
    pub(crate) fn set_payload_length(buf: &mut [u8], payload_length: usize) {
        let payload_length_with_seq_num = BigEndian::read_u64(&buf[24..32]);
        let payload_length_with_seq_num =
            (payload_length_with_seq_num & BIT_MASK_RIGHT_50) | ((payload_length as u64) << 50);
        BigEndian::write_u64(&mut buf[24..32], payload_length_with_seq_num);
    }

    #[inline]
    pub(crate) fn set_timestamp(buf: &mut [u8], timestamp: u64) {
        let type_extension_with_timestamp = BigEndian::read_u64(&buf[16..24]);
        let type_extension_with_timestamp =
            (type_extension_with_timestamp & BIT_MASK_LEFT_46) | (timestamp & BIT_MASK_RIGHT_46);
        BigEndian::write_u64(&mut buf[16..24], type_extension_with_timestamp);
    }

    #[inline]
    pub(crate) fn set_seq_num(buf: &mut [u8], seq_num: u64) {
        let payload_length_with_seq_num = BigEndian::read_u64(&buf[24..32]);
        let payload_length_with_seq_num =
            (payload_length_with_seq_num & BIT_MASK_LEFT_50) | (seq_num & BIT_MASK_RIGHT_50);
        BigEndian::write_u64(&mut buf[24..32], payload_length_with_seq_num);
    }
}

impl From<&Head> for InnerHead {
    fn from(head: &Head) -> Self {
        let version = (head.version_with_sender >> 46) as u32;
        let sender = head.version_with_sender & BIT_MASK_RIGHT_46;
        let node_id = (head.node_id_with_receiver >> 46) as u32;
        let receiver = head.node_id_with_receiver & BIT_MASK_RIGHT_46;
        let typ = (head.type_with_extension_length_with_timestamp >> 52) as u16;
        let extension_length =
            ((head.type_with_extension_length_with_timestamp & BIT_MASK_RIGHT_12) >> 46) as u8;
        let timestamp = head.type_with_extension_length_with_timestamp & BIT_MASK_RIGHT_46;
        let payload_length = (head.payload_length_with_seq_num >> 50) as u16;
        let seq_num = head.payload_length_with_seq_num & BIT_MASK_RIGHT_50;
        Self {
            version,
            sender,
            node_id,
            receiver,
            typ: Type::from(typ),
            extension_length,
            payload_length,
            timestamp,
            seq_num,
        }
    }
}

impl Into<Head> for InnerHead {
    fn into(self) -> Head {
        let version_with_sender = ((self.version as u64) << 46) | self.sender;
        let node_id_with_receiver = ((self.node_id as u64) << 46) | self.receiver;
        let type_with_extension_length_with_timestamp = ((self.typ.value() as u64) << 52)
            | ((self.extension_length as u64) << 46)
            | self.timestamp;
        let payload_length_with_seq_num = ((self.payload_length as u64) << 50) | self.seq_num;
        Head {
            version_with_sender,
            node_id_with_receiver,
            type_with_extension_length_with_timestamp,
            payload_length_with_seq_num,
        }
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

impl Msg {
    #[inline]
    pub fn pre_alloc(head: &mut Head) -> Self {
        let extension_length =
            ((head.type_with_extension_length_with_timestamp & BIT_MASK_RIGHT_12) >> 46) as usize;
        let payload_length = (head.payload_length_with_seq_num >> 50) as usize;
        let mut buf = Vec::with_capacity(HEAD_LEN + payload_length + extension_length);
        unsafe {
            buf.set_len(HEAD_LEN + payload_length + extension_length);
        }
        let _ = head.read(buf.as_mut_slice());
        Self(buf)
    }

    #[inline]
    pub fn pre_allocate(payload_length: usize, extension_length: usize) -> Self {
        let inner_head = InnerHead {
            extension_length: extension_length as u8,
            payload_length: extension_length as u16,
            typ: Type::NA,
            sender: 0,
            receiver: 0,
            node_id: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut head: Head = inner_head.into();
        let mut buf = Vec::with_capacity(HEAD_LEN + payload_length + extension_length);
        unsafe {
            buf.set_len(HEAD_LEN + payload_length + extension_length);
        }
        let _ = head.read(buf.as_mut_slice());
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
        Head::extension_length(self.as_slice())
    }

    #[inline]
    pub fn payload_length(&self) -> usize {
        Head::payload_length(self.as_slice())
    }

    #[inline]
    pub fn typ(&self) -> Type {
        Head::typ(self.as_slice())
    }

    #[inline]
    pub fn sender(&self) -> u64 {
        Head::sender(self.as_slice())
    }

    #[inline]
    pub fn receiver(&self) -> u64 {
        Head::receiver(self.as_slice())
    }

    #[inline]
    pub fn node_id(&self) -> u32 {
        Head::node_id(self.as_slice())
    }

    #[inline]
    pub fn timestamp(&self) -> u64 {
        Head::timestamp(self.as_slice())
    }

    #[inline]
    pub fn seq_num(&self) -> u64 {
        Head::seq_num(self.as_slice())
    }

    #[allow(unused)]
    #[inline]
    pub fn version(&self) -> u32 {
        Head::version(self.as_slice())
    }

    #[inline]
    pub fn set_extension_length(&mut self, extension_length: usize) {
        Head::set_extension_length(self.as_mut_slice(), extension_length);
    }

    #[inline]
    pub fn set_payload_length(&mut self, payload_length: usize) {
        Head::set_payload_length(self.as_mut_slice(), payload_length);
    }

    #[inline]
    pub fn set_type(&mut self, typ: Type) {
        Head::set_type(self.as_mut_slice(), typ);
    }

    #[inline]
    pub fn set_sender(&mut self, sender: u64) {
        Head::set_sender(self.as_mut_slice(), sender);
    }

    #[inline]
    pub fn set_receiver(&mut self, receiver: u64) {
        Head::set_receiver(self.as_mut_slice(), receiver);
    }

    #[inline]
    pub fn set_node_id(&mut self, sender_node: u32) {
        Head::set_node_id(self.as_mut_slice(), sender_node);
    }

    #[inline]
    pub fn set_timestamp(&mut self, timestamp: u64) {
        Head::set_timestamp(self.as_mut_slice(), timestamp);
    }

    #[allow(unused)]
    #[inline]
    pub fn set_seq_num(&mut self, seq_num: u64) {
        Head::set_seq_num(self.as_mut_slice(), seq_num);
    }

    #[allow(unused)]
    #[inline]
    pub fn set_version(&mut self, version: u32) {
        Head::set_version(self.as_mut_slice(), version);
    }

    #[allow(unused)]
    #[inline]
    pub fn extension(&self) -> &[u8] {
        let extension_length = self.extension_length();
        let payload_length = self.payload_length();
        if extension_length == 0 {
            &[]
        } else {
            &self.as_slice()
                [HEAD_LEN + payload_length..HEAD_LEN + payload_length + extension_length]
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn extension_mut(&mut self) -> &mut [u8] {
        let extension_length = self.extension_length();
        let payload_length = self.payload_length();
        if extension_length == 0 {
            &mut []
        } else {
            &mut self.as_mut_slice()
                [HEAD_LEN + payload_length..HEAD_LEN + payload_length + extension_length]
        }
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        let payload_length = self.payload_length();
        if payload_length == 0 {
            &[]
        } else {
            &self.as_slice()[HEAD_LEN..HEAD_LEN + payload_length]
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let payload_length = self.payload_length();
        if payload_length == 0 {
            &mut []
        } else {
            &mut self.as_mut_slice()[HEAD_LEN..HEAD_LEN + payload_length]
        }
    }

    #[allow(unused)]
    #[inline]
    /// can work only on new payload has same length with old payload
    pub fn set_payload(&mut self, payload: &[u8]) -> bool {
        let payload_length = payload.len();
        if payload_length != payload.len() {
            return false;
        }
        self.as_mut_slice()[HEAD_LEN..HEAD_LEN + payload_length].copy_from_slice(payload);
        true
    }

    #[allow(unused)]
    #[inline]
    /// can work only on new extension has same length with old extension
    pub fn set_extension(&mut self, extension: &[u8]) -> bool {
        let extension_length = extension.len();
        let payload_length = self.payload_length();
        if extension_length != extension.len() {
            return false;
        }
        self.as_mut_slice()
            [HEAD_LEN + payload_length..HEAD_LEN + payload_length + extension_length]
            .copy_from_slice(extension);
        true
    }

    #[allow(unused)]
    #[inline]
    pub fn ping(sender: u64, receiver: u64, node_id: u32) -> Self {
        let inner_head = InnerHead {
            extension_length: 0,
            payload_length: 4,
            typ: Type::Ping,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(b"ping");
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn pong(sender: u64, receiver: u64, node_id: u32) -> Self {
        let inner_head = InnerHead {
            extension_length: 0,
            payload_length: 4,
            typ: Type::Ping,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(b"pong");
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn err_msg(sender: u64, receiver: u64, node_id: u32, reason: &str) -> Self {
        let mut inner_head = InnerHead {
            extension_length: 0,
            payload_length: reason.len() as u16,
            typ: Type::Error,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(reason.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn text(sender: u64, receiver: u64, node_id: u32, text: &str) -> Self {
        let mut inner_head = InnerHead {
            extension_length: 0,
            payload_length: text.len() as u16,
            typ: Type::Text,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(text.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn text2(sender: u64, receiver: u64, node_id: u32, text: &str, text2: &str) -> Self {
        let mut inner_head = InnerHead {
            extension_length: text2.len() as u8,
            payload_length: text.len() as u16,
            typ: Type::Text,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(
            HEAD_LEN + inner_head.payload_length as usize + inner_head.extension_length as usize,
        );
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(text.as_bytes());
        buf.extend_from_slice(text2.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn generate_ack(&self) -> Self {
        let time = self.timestamp().to_string();
        let inner_head = InnerHead {
            extension_length: 0,
            payload_length: time.len() as u16,
            typ: Type::Ack,
            sender: 0,
            receiver: self.sender(),
            node_id: 0,
            timestamp: timestamp(),
            seq_num: self.seq_num(),
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(time.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn ack(client_timestamp: u64) -> Self {
        let time = client_timestamp.to_string();
        let inner_head = InnerHead {
            extension_length: 0,
            payload_length: time.len() as u16,
            typ: Type::Ack,
            sender: 0,
            receiver: 0,
            node_id: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(time.as_bytes());
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn empty() -> Self {
        let mut inner_head = InnerHead {
            extension_length: 0,
            payload_length: 0,
            typ: Type::NA,
            sender: 0,
            receiver: 0,
            node_id: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut head: Head = inner_head.into();
        let mut buf = Vec::with_capacity(HEAD_LEN);
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn auth(sender: u64, receiver: u64, node_id: u32, token: &str) -> Self {
        let token = token.as_bytes();
        let mut inner_head = InnerHead {
            extension_length: 0,
            payload_length: token.len() as u16,
            typ: Type::Auth,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(token);
        Self(buf)
    }

    #[allow(unused)]
    #[inline]
    pub fn raw_payload(payload: &Vec<u8>) -> Self {
        let mut inner_head = InnerHead {
            extension_length: 0,
            payload_length: payload.len() as u16,
            typ: Type::NA,
            sender: 0,
            receiver: 0,
            node_id: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        head.read(&mut buf);
        buf.extend_from_slice(payload);
        Self(buf)
    }

    pub fn raw(sender: u64, receiver: u64, node_id: u32, payload: &[u8]) -> Self {
        let inner_head = InnerHead {
            extension_length: 0,
            payload_length: payload.len() as u16,
            typ: Type::NA,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(HEAD_LEN + inner_head.payload_length as usize);
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        _ = head.read(&mut buf);
        buf.extend_from_slice(payload);
        Self(buf)
    }

    pub fn raw2(
        sender: u64,
        receiver: u64,
        node_id: u32,
        payload: &[u8],
        extension: &[u8],
    ) -> Self {
        let inner_head = InnerHead {
            extension_length: extension.len() as u8,
            payload_length: payload.len() as u16,
            typ: Type::NA,
            sender,
            receiver,
            node_id,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(
            HEAD_LEN + inner_head.payload_length as usize + inner_head.extension_length as usize,
        );
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        _ = head.read(&mut buf);
        buf.extend_from_slice(payload);
        buf.extend_from_slice(extension);
        Self(buf)
    }

    pub fn noop() -> Self {
        let mut empty = Self::empty();
        empty.set_type(Type::Noop);
        empty
    }

    pub fn from_payload_extension(payload: &[u8], extension: &[u8]) -> Self {
        let inner_head = InnerHead {
            extension_length: extension.len() as u8,
            payload_length: payload.len() as u16,
            typ: Type::NA,
            sender: 0,
            receiver: 0,
            node_id: 0,
            timestamp: timestamp(),
            seq_num: 0,
            version: 0,
        };
        let mut buf = Vec::with_capacity(
            HEAD_LEN + inner_head.payload_length as usize + inner_head.extension_length as usize,
        );
        let mut head: Head = inner_head.into();
        unsafe {
            buf.set_len(HEAD_LEN);
        }
        let _ = head.read(&mut buf);
        buf.extend_from_slice(payload);
        buf.extend_from_slice(extension);
        Self(buf)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use crate::entity::{Head, InnerHead, Msg, Type};

    #[test]
    fn test() {
        let head = InnerHead {
            version: 6,
            sender: 1,
            node_id: 3,
            receiver: 2,
            typ: Type::Ack,
            extension_length: 8,
            timestamp: 4,
            payload_length: 7,
            seq_num: 5,
        };
        let mut h: Head = head.into();
        let mut arr = [0u8; 32];
        let _ = h.read(&mut arr);
        println!("{:?}", arr);
        let mut buf = Vec::with_capacity(32);
        unsafe { buf.set_len(32) };
        let _ = h.read(&mut buf);
        println!("{}", Head::sender(&buf));
        println!("{}", Head::receiver(&buf));
        println!("{}", Head::node_id(&buf));
        println!("{}", Head::timestamp(&buf));
        println!("{}", Head::seq_num(&buf));
        println!("{}", Head::version(&buf));
        println!("{}", Head::payload_length(&buf));
        println!("{}", Head::extension_length(&buf));
        println!("{}", Head::typ(&buf));
        Head::set_sender(&mut buf, 11);
        Head::set_receiver(&mut buf, 12);
        Head::set_node_id(&mut buf, 13);
        Head::set_timestamp(&mut buf, 14);
        Head::set_seq_num(&mut buf, 15);
        Head::set_version(&mut buf, 16);
        Head::set_payload_length(&mut buf, 17);
        Head::set_extension_length(&mut buf, 18);
        Head::set_type(&mut buf, Type::Text);
        println!("{}", Head::sender(&buf));
        println!("{}", Head::receiver(&buf));
        println!("{}", Head::node_id(&buf));
        println!("{}", Head::timestamp(&buf));
        println!("{}", Head::seq_num(&buf));
        println!("{}", Head::version(&buf));
        println!("{}", Head::payload_length(&buf));
        println!("{}", Head::extension_length(&buf));
        println!("{}", Head::typ(&buf));
        let msg = Msg::text(1, 2, 3, "一只狗");
        println!("{:?}", msg.as_bytes());
    }
}
