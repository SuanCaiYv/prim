use std::{
    fmt::{Display, Formatter},
    ops::Add,
    time::{Duration, SystemTime},
};

use base64::Engine;
use chrono::{DateTime, Local};
use lib::{
    entity::{Msg, Type},
    Result,
};
use sqlx::Postgres;

use crate::sql::get_sql_pool;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow)]
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
}

impl From<&Msg> for Message {
    fn from(msg: &Msg) -> Self {
        let t: DateTime<Local> = DateTime::from(
            SystemTime::UNIX_EPOCH.add(Duration::from_millis(msg.timestamp() as u64)),
        );
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        Self {
            id: 0,
            sender: msg.sender() as i64,
            receiver: msg.receiver() as i64,
            timestamp: t,
            seq_num: msg.seqnum() as i64,
            typ: msg.typ(),
            version: msg.version() as i16,
            extension: engine.encode(String::from_utf8_lossy(msg.extension()).to_string()),
            payload: engine.encode(String::from_utf8_lossy(msg.payload()).to_string()),
        }
    }
}

impl Into<Msg> for &Message {
    fn into(self) -> Msg {
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        let extension = self.extension.as_bytes();
        let mut extension = engine
            .decode(extension)
            .unwrap_or(Vec::from("base64 decode fatal"));
        let payload = self.payload.as_bytes();
        let mut payload = engine
            .decode(payload)
            .unwrap_or(Vec::from("base64 decode fatal"));
        let mut msg = Msg::pre_allocate(extension.len(), payload.len());
        msg.set_extension_length(extension.len());
        msg.set_payload_length(payload.len());
        msg.set_type(self.typ);
        msg.set_sender(self.sender as u64);
        msg.set_receiver(self.receiver as u64);
        msg.set_timestamp(self.timestamp.timestamp_millis() as u64);
        msg.set_seqnum(self.seq_num as u64);
        msg.set_version(self.version as u32);
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
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::PAD,
        );
        write!(f, "Message {{ id: {}, sender: {}, receiver: {}, timestamp: {}, seq_num: {}, typ: {:?}, version: {}, extension: {}, payload: {} }}",
               self.id,
               self.sender,
               self.receiver,
               self.timestamp,
               self.seq_num,
               self.typ,
               self.version,
               String::from_utf8_lossy(engine.decode(self.extension.as_bytes()).unwrap().as_slice()),
               String::from_utf8_lossy(engine.decode(self.payload.as_bytes()).unwrap().as_slice())
        )
    }
}

impl Message {
    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO msg.message (sender, receiver, timestamp, seq_num, type, version, extension, payload) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(self.sender)
            .bind(self.receiver)
            .bind(self.timestamp)
            .bind(self.seq_num)
            .bind(self.typ.value() as i16)
            .bind(self.version)
            .bind(&self.extension)
            .bind(&self.payload)
            .execute(get_sql_pool().await).await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE msg.message SET sender = $1, receiver = $2, timestamp = $3, seq_num = $4, type = $5, version = $6, extension = $7, payload = $8, where id = $9")
            .bind(&self.sender)
            .bind(&self.receiver)
            .bind(&self.timestamp)
            .bind(&self.seq_num)
            .bind(&(self.typ.value() as i16))
            .bind(&self.version)
            .bind(&self.extension)
            .bind(&self.payload)
            .bind(&self.id)
            .execute(get_sql_pool().await).await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("DELETE FROM msg.message WHERE id = $1")
            .bind(self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn get(id: i64) -> Result<Self> {
        let msg = sqlx::query_as("SELECT id, sender, receiver, timestamp, seq_num, type, version, extension, payload FROM msg.message WHERE id = $1")
            .bind(id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(msg)
    }

    #[allow(unused)]
    pub(crate) async fn get_by_user_and_peer(
        user_id: i64,
        peer_id: i64,
        from_seq: i64,
        to_seq: i64,
    ) -> Result<Vec<Self>> {
        if to_seq == i64::MAX || (to_seq as u64) == u64::MAX {
            let msgs = sqlx::query_as("SELECT id, sender, receiver, timestamp, seq_num, type, version, extension, payload FROM msg.message WHERE (sender = $1 AND receiver = $2 OR sender = $2 AND receiver = $1) ORDER BY seq_num DESC LIMIT $3")
                .bind(&user_id)
                .bind(&peer_id)
                .bind(&(to_seq - from_seq))
                .fetch_all(get_sql_pool().await)
                .await?;
            Ok(msgs)
        } else {
            let msgs = sqlx::query_as("SELECT id, sender, receiver, timestamp, seq_num, type, version, extension, payload FROM msg.message WHERE (sender = $1 AND receiver = $2 OR sender = $2 AND receiver = $1) AND seq_num >= $3 AND seq_num < $4 ORDER BY seq_num DESC")
                .bind(&user_id)
                .bind(&peer_id)
                .bind(&from_seq)
                .bind(&to_seq)
                .fetch_all(get_sql_pool().await)
                .await?;
            Ok(msgs)
        }
    }

    #[allow(unused)]
    pub(crate) async fn insert_batch(msg_list: Vec<Message>) -> Result<()> {
        let mut batch_inserter: sqlx::QueryBuilder<Postgres> = sqlx::QueryBuilder::new("INSERT INTO msg.message (sender, receiver, timestamp, seq_num, type, version, extension, payload) ");
        batch_inserter.push_values(msg_list, |mut binder, msg| {
            binder.push_bind(msg.sender);
            binder.push_bind(msg.receiver);
            binder.push_bind(msg.timestamp);
            binder.push_bind(msg.seq_num);
            binder.push_bind((msg.typ.value() as i16));
            binder.push_bind(msg.version);
            binder.push_bind(msg.extension);
            binder.push_bind(msg.payload);
        });
        let query = batch_inserter.build();
        query.execute(get_sql_pool().await).await?;
        Ok(())
    }
}
