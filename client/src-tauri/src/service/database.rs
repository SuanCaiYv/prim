use std::path::PathBuf;

use lib::entity::Msg;
use lib::Result;
use rusqlite::params;
use tokio_rusqlite::Connection;
use tracing::error;

use crate::LOCAL_DATA_DIR;

const MSG_DB_CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS msg (
    id          INTEGER PRIMARY KEY,
    sender      INTEGER,
    receiver    INTEGER,
    \"timestamp\" INTEGER,
    seq_num     INTEGER,
    type        INTEGER,
    version     INTEGER,
    payload     TEXT,
    extension   TEXT
)";

const KV_DB_CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS kv (
    id          INTEGER PRIMARY KEY,
    key         TEXT,
    value       BLOB
)";

/// only save acknowledged msg
pub(crate) struct MsgDB {
    connection: Connection,
}

impl MsgDB {
    pub(crate) async fn new() -> Self {
        let mut path = PathBuf::from(unsafe { LOCAL_DATA_DIR });
        path.push("prim_msg.sqlite");
        let connection = Connection::open(path).await.unwrap();
        connection
            .call(|conn| {
                let mut stmt = conn.prepare(MSG_DB_CREATE_TABLE).unwrap();
                stmt.execute(params![]).unwrap();
            })
            .await;
        Self { connection }
    }

    pub(self) async fn insert(&self, msg: Msg) -> Result<()> {
        self.connection
            .call(move |conn| {
                conn
                    .execute(
                        "INSERT INTO msg (sender, receiver, \"timestamp\", seq_num, type, version, payload, extension) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![msg.sender(), msg.receiver(), msg.timestamp(), msg.seq_num(), msg.typ(), msg.version(), String::from_utf8_lossy(msg.payload()).to_string(), String::from_utf8_lossy(msg.extension()).to_string()]
                    )?;
                    Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(self) async fn update(&self, msg: Msg) -> Result<()> {
        self.connection
            .call(move |conn| {
                conn
                    .execute(
                        "UPDATE msg SET \"timestamp\" = ?1, type = ?2, version = ?3, payload = ?4, extension = ?5 WHERE sender = ?6 AND receiver = ?7 AND seq_num = ?8" ,
                        params![msg.timestamp(), msg.typ(), msg.version(), String::from_utf8_lossy(msg.payload()).to_string(), String::from_utf8_lossy(msg.extension()).to_string(), msg.sender(), msg.receiver(), msg.seq_num()]
                    )?;
                    Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(self) async fn latest(&self, user_id1: u64, user_id2: u64) -> Result<Option<Msg>> {
        let res = self.connection.call(move |conn| {
            let mut statement = conn.prepare("SELECT sender, receiver, \"timestamp\", seq_num, type, version, payload, extension FROM msg WHERE ((sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1)) ORDER BY seq_num DESC LIMIT 1")?;
            let res = statement
                .query_map(params![user_id1, user_id2], |row| {
                    let sender: u64 = row.get(0)?;
                    let receiver: u64 = row.get(1)?;
                    let timestamp: u64 = row.get(2)?;
                    let seq_num: u64 = row.get(3)?;
                    let typ: u16 = row.get(4)?;
                    let version: u32 = row.get(5)?;
                    let payload: String = row.get(6)?;
                    let extension: String = row.get(7)?;
                    let mut msg = Msg::raw2(sender, receiver, 0, payload.as_bytes(), extension.as_bytes());
                    msg.set_timestamp(timestamp);
                    msg.set_seq_num(seq_num);
                    msg.set_type(typ.into());
                    msg.set_version(version);
                    Ok(msg)
                })?
                .collect::<std::result::Result<Vec<Msg>, rusqlite::Error>>()?;
            if res.len() == 0 {
                Ok::<Option<Msg>, rusqlite::Error>(None)
            } else {
                Ok::<Option<Msg>, rusqlite::Error>(Some(res[0].clone()))
            }
        }).await?;
        Ok(res)
    }

    pub(crate) async fn insert_or_update_list(&self, msg_list: Vec<Msg>) -> Result<()> {
        for msg in msg_list.into_iter() {
            if let Some(_) = self
                .select(msg.sender(), msg.receiver(), msg.seq_num())
                .await?
            {
                self.update(msg).await?;
            } else {
                self.insert(msg).await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn insert_or_update(&self, msg: Msg) -> Result<()> {
        if let Some(_) = self
            .select(msg.sender(), msg.receiver(), msg.seq_num())
            .await?
        {
            self.update(msg).await?;
        } else {
            self.insert(msg).await?;
        }
        Ok(())
    }

    pub(crate) async fn select(
        &self,
        user_id1: u64,
        user_id2: u64,
        seq_num: u64,
    ) -> Result<Option<Msg>> {
        let res = self.connection.call(move |conn| {
            let mut statement = conn.prepare("SELECT sender, receiver, \"timestamp\", seq_num, type, version, payload, extension FROM msg WHERE ((sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1)) AND seq_num = ?3")?;
            let res = statement
                .query_map(params![user_id1, user_id2, seq_num], |row| {
                    let sender: u64 = row.get(0)?;
                    let receiver: u64 = row.get(1)?;
                    let timestamp: u64 = row.get(2)?;
                    let seq_num: u64 = row.get(3)?;
                    let typ: u16 = row.get(4)?;
                    let version: u32 = row.get(5)?;
                    let payload: String = row.get(6)?;
                    let extension: String = row.get(7)?;
                    let mut msg = Msg::raw2(sender, receiver, 0, payload.as_bytes(), extension.as_bytes());
                    msg.set_timestamp(timestamp);
                    msg.set_seq_num(seq_num);
                    msg.set_type(typ.into());
                    msg.set_version(version);
                    Ok(msg)
                })?
                .collect::<std::result::Result<Vec<Msg>, rusqlite::Error>>()?;
            if res.len() == 0 {
                Ok::<Option<Msg>, rusqlite::Error>(None)
            } else {
                Ok::<Option<Msg>, rusqlite::Error>(Some(res[0].clone()))
            }
        }).await?;
        Ok(res)
    }

    pub(crate) async fn find_list(
        &self,
        user_id1: u64,
        user_id2: u64,
        seq_num_from: u64,
        seq_num_to: u64,
    ) -> Result<Option<Vec<Msg>>> {
        let res = self.connection.call(move |conn| {
            let mut statement = conn.prepare("SELECT sender, receiver, \"timestamp\", seq_num, type, version, payload, extension FROM msg WHERE ((sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1)) AND seq_num >= ?3 AND seq_num < ?4 ORDER BY seq_num DESC")?;
            let res = statement
                .query_map(params![user_id1, user_id2, seq_num_from, seq_num_to], |row| {
                    let sender: u64 = row.get(0)?;
                    let receiver: u64 = row.get(1)?;
                    let timestamp: u64 = row.get(2)?;
                    let seq_num: u64 = row.get(3)?;
                    let typ: u16 = row.get(4)?;
                    let version: u32 = row.get(5)?;
                    let payload: String = row.get(6)?;
                    let extension: String = row.get(7)?;
                    let mut msg = Msg::raw2(sender, receiver, 0, payload.as_bytes(), extension.as_bytes());
                    msg.set_timestamp(timestamp);
                    msg.set_seq_num(seq_num);
                    msg.set_type(typ.into());
                    msg.set_version(version);
                    Ok(msg)
                })?
                .collect::<std::result::Result<Vec<Msg>, rusqlite::Error>>()?;
            if res.len() == 0 {
                Ok::<Option<Vec<Msg>>, rusqlite::Error>(None)
            } else {
                Ok::<Option<Vec<Msg>>, rusqlite::Error>(Some(res))
            }
        }).await?;
        Ok(res)
    }

    pub(self) async fn delete(&self, user_id1: u64, user_id2: u64, seq_num: u64) -> Result<()> {
        self.connection
            .call(move |conn| {
                conn
                    .execute(
                        "DELETE FROM msg WHERE ((sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1)) AND seq_num = ?3",
                        params![user_id1, user_id2, seq_num],
                    )?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_list(
        &self,
        user_id1: u64,
        user_id2: u64,
        seq_num_list: &[u64],
    ) -> Result<()> {
        for seq_num in seq_num_list {
            self.delete(user_id1, user_id2, *seq_num).await?;
        }
        Ok(())
    }

    pub(crate) async fn latest_seq_num(&self, user_id1: u64, user_id2: u64) -> Result<Option<u64>> {
        let res = self.connection.call(move |conn| {
            let mut statement = conn.prepare("SELECT seq_num FROM msg WHERE ((sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1)) ORDER BY seq_num DESC LIMIT 1")?;
            let res = statement
                .query_map(params![user_id1, user_id2], |row| {
                    let seq_num: u64 = row.get(0)?;
                    Ok(seq_num)
                })?
                .collect::<std::result::Result<Vec<u64>, rusqlite::Error>>()?;
            if res.len() == 0 {
                Ok::<Option<u64>, rusqlite::Error>(None)
            } else {
                Ok::<Option<u64>, rusqlite::Error>(Some(res[0]))
            }
        }).await?;
        Ok(res)
    }
}

/// only accept js object in string
pub(crate) struct KVDB {
    connection: Connection,
}

impl KVDB {
    pub(crate) async fn new() -> Self {
        let mut path = PathBuf::from(unsafe { LOCAL_DATA_DIR });
        path.push("prim_kv.sqlite");
        let connection = Connection::open(path).await.unwrap();
        connection
            .call(|conn| {
                let mut stmt = conn.prepare(KV_DB_CREATE_TABLE).unwrap();
                stmt.execute(params![]).unwrap();
            })
            .await;
        Self { connection }
    }

    pub(self) async fn insert(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let key = key.to_owned();
        let value = value.to_owned();
        self.connection
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO kv (key, value) VALUES (?1, ?2)",
                    params![key, value.to_string().as_bytes()],
                )?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(self) async fn update(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let key = key.to_owned();
        let value = value.to_owned();
        self.connection
            .call(move |conn| {
                conn.execute(
                    "UPDATE kv SET value = ?2 WHERE key = ?1",
                    params![key, value.to_string().as_bytes()],
                )?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(self) async fn select(&self, key: &str) -> Result<serde_json::Value> {
        let key = key.to_owned();
        let s = self
            .connection
            .call(move |conn| {
                let mut statement = conn.prepare("SELECT value FROM KV WHERE key = ?1")?;
                let res = statement
                    .query_map(params![key], |row| {
                        let value: Vec<u8> = row.get(0)?;
                        let value = serde_json::from_slice(&value);
                        match value {
                            Ok(value) => Ok(value),
                            Err(e) => {
                                error!("kvdb select error: {}", e);
                                Err(rusqlite::Error::InvalidQuery)
                            }
                        }
                    })?
                    .collect::<std::result::Result<Vec<serde_json::Value>, rusqlite::Error>>()?;
                if res.len() == 0 {
                    Err(rusqlite::Error::QueryReturnedNoRows)
                } else {
                    Ok::<serde_json::Value, rusqlite::Error>(res[0].clone())
                }
            })
            .await?;
        Ok(s)
    }

    pub(self) async fn delete(&self, key: &str) -> Result<()> {
        let key = key.to_owned();
        self.connection
            .call(move |conn| {
                conn.execute("DELETE FROM kv WHERE key = ?1", params![key])?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(crate) async fn set(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        match self.select(key).await {
            Ok(val) => {
                self.update(key, value).await?;
                Ok(Some(val))
            }
            Err(_) => {
                self.insert(key, value).await?;
                Ok(None)
            }
        }
    }

    pub(crate) async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        match self.select(key).await {
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None),
        }
    }

    pub(crate) async fn del(&self, key: &str) -> Result<Option<serde_json::Value>> {
        match self.select(key).await {
            Ok(val) => {
                self.delete(key).await?;
                Ok(Some(val))
            }
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {
        // let data = json!({
        //     "aaa": "aaa",
        //     "bbb": {
        //         "ccc": [1, 2, 3]
        //     }
        // });
        // let db = KVDB::new().await;
        // db.set("test", &data).await;
        // let res = db.get("test").await;
        // println!("{}", res.unwrap().unwrap().to_string());
    }
}
