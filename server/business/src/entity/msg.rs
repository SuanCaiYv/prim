use crate::SQL_POOL;
use chrono::{DateTime, Local};
use common::entity::{Msg, Type};
use common::Result;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::time::{Duration, SystemTime};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow, Default)]
pub(crate) struct Message {
    pub(crate) id: i64,
    pub(crate) sender: i64,
    pub(crate) receiver: i64,
    pub(crate) timestamp: DateTime<Local>,
    pub(crate) seq_num: i64,
    #[sqlx(rename = "type", try_from = "i16")]
    pub(crate) typ: Type,
    pub(crate) version: i16,
    pub(crate) extension: String,
    pub(crate) payload: String,
    pub(crate) status: i16,
}

impl From<&Msg> for Message {
    fn from(msg: &Msg) -> Self {
        let t: DateTime<Local> = DateTime::from(
            SystemTime::UNIX_EPOCH.add(Duration::from_millis(msg.timestamp() as u64)),
        );
        Self {
            id: 0,
            sender: msg.sender() as i64,
            receiver: msg.receiver() as i64,
            timestamp: t,
            seq_num: msg.seq_num() as i64,
            typ: msg.typ(),
            version: msg.version() as i16,
            extension: base64::encode(String::from_utf8_lossy(msg.extension()).to_string()),
            payload: base64::encode(String::from_utf8_lossy(msg.payload()).to_string()),
            status: 1,
        }
    }
}

impl Into<Msg> for Message {
    fn into(self) -> Msg {
        let extension = self.extension.as_bytes();
        let mut extension = base64::decode(extension).unwrap_or(Vec::from("base64 decode fatal"));
        let payload = self.payload.as_bytes();
        let mut payload = base64::decode(payload).unwrap_or(Vec::from("base64 decode fatal"));
        let mut msg = Msg::pre_alloc(extension.len() as u16, payload.len() as u16);
        msg.update_extension_length(extension.len() as u16);
        msg.update_payload_length(payload.len() as u16);
        msg.update_type(self.typ);
        msg.update_sender(self.sender as u64);
        msg.update_receiver(self.receiver as u64);
        msg.update_timestamp(self.timestamp.timestamp_millis() as u64);
        msg.update_seq_num(self.seq_num as u64);
        msg.update_version(self.version as u16);
        unsafe {
            std::ptr::copy(
                extension.as_mut_ptr(),
                msg.extension_mut().as_mut_ptr(),
                extension.len(),
            )
        };
        unsafe {
            std::ptr::copy(
                payload.as_mut_ptr(),
                msg.payload_mut().as_mut_ptr(),
                payload.len(),
            )
        };
        msg
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Message {{ id: {}, sender: {}, receiver: {}, timestamp: {}, seq_num: {}, typ: {:?}, version: {}, extension: {}, payload: {}, status: {} }}",
            self.id,
            self.sender,
            self.receiver,
            self.timestamp,
            self.seq_num,
            self.typ,
            self.version,
            String::from_utf8_lossy(base64::decode(self.extension.as_bytes()).unwrap().as_slice()),
            String::from_utf8_lossy(base64::decode(self.payload.as_bytes()).unwrap().as_slice()),
            self.status
        )
    }
}

impl Message {
    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("insert into msg.message (sender, receiver, timestamp, seq_num, type, version, extension, payload, status) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(self.sender)
            .bind(self.receiver)
            .bind(self.timestamp)
            .bind(self.seq_num)
            .bind(self.typ.values() as i16)
            .bind(self.version)
            .bind(&self.extension)
            .bind(&self.payload)
            .bind(self.status)
            .execute(unsafe {&*SQL_POOL.as_ref().unwrap()}).await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("update msg.message set sender = $1, receiver = $2, timestamp = $3, seq_num = $4, type = $5, version = $6, extension = $7, payload = $8, status = $9 where id = $10")
            .bind(self.sender)
            .bind(self.receiver)
            .bind(self.timestamp)
            .bind(self.seq_num)
            .bind(self.typ.values() as i16)
            .bind(self.version)
            .bind(&self.extension)
            .bind(&self.payload)
            .bind(self.status)
            .bind(self.id)
            .execute(unsafe {&*SQL_POOL.as_ref().unwrap()}).await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("delete from msg.message where id = $1")
            .bind(self.id)
            .execute(unsafe { &*SQL_POOL.as_ref().unwrap() })
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn get(id: i64) -> Result<Self> {
        let msg = sqlx::query_as("select * from msg.message where id = $1")
            .bind(id)
            .fetch_one(unsafe { &*SQL_POOL.as_ref().unwrap() })
            .await?;
        Ok(msg)
    }
}
