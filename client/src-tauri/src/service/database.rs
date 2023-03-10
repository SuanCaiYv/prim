use lib::entity::Msg;
use lib::Result;
use rusqlite::params;
use tokio_rusqlite::Connection;

const MSG_DB_CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS msg (
    id          INTEGER PRIMARY KEY,
    sender      INTEGER,
    receiver    INTEGER,
    \"timestamp\" INTEGER,
    seq_num     INTEGER,
    type        INTEGER,
    version     INTEGER,
    extension   TEXT,
    payload     TEXT
)";

const KV_DB_CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS kv (
    id          INTEGER PRIMARY KEY,
    key         TEXT,
    value       TEXT
)";

/// only save acknowledged msg
pub(crate) struct MsgDB {
    connection: Connection,
}

impl MsgDB {
    pub(crate) async fn new() -> Self {
        let connection = Connection::open("prim_msg.db").await.unwrap();
        connection
            .call(|conn| {
                let mut stmt = conn.prepare(MSG_DB_CREATE_TABLE).unwrap();
                stmt.execute(params![]).unwrap();
            })
            .await;
        Self { connection }
    }

    pub(self) async fn insert(&self, msg: &Msg) -> Result<()> {
        let msg = msg.clone();
        self.connection
            .call(move |conn| {
                conn
                    .execute(
                        "INSERT INTO msg (
                            sender,
                            receiver,
                            \"timestamp\",
                            seq_num,
                            type,
                            version,
                            extension,
                            payload
                        ) VALUES (
                            ?1,
                            ?2,
                            ?3,
                            ?4,
                            ?5,
                            ?6,
                            ?7,
                            ?8
                        )", params![msg.sender(), msg.receiver(), msg.timestamp(), msg.seq_num(), msg.typ(), msg.version(), msg.extension(), msg.payload()]
                    )?;
                    Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub(self) async fn update(&self, msg: &Msg) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn insert_or_update(&self, msg_list: &[Msg]) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn select(&self, user_id1: u64, user_id2: u64, seq_num: u64) -> Result<Option<Msg>> {
        Ok(None)
    }

    pub(crate) async fn find_list(&self, user_id1: u64, user_id2: u64, seq_num_from: u64, seq_num_to: u64) -> Result<Option<Msg>> {
        Ok(None)
    }

    pub(crate) async fn delete_list(&self, user_id1: u64, user_id2: u64, seq_num_list: &[u64]) -> Result<()> {
        Ok(())
    }
}

/// only accept js object in string
pub(crate) struct KVDB {
    connection: Connection,
}

impl KVDB {
    pub(crate) async fn new() -> Self {
        let connection = Connection::open("prim_kv.db").await.unwrap();
        connection
            .call(|conn| {
                let mut stmt = conn.prepare(KV_DB_CREATE_TABLE).unwrap();
                stmt.execute(params![]).unwrap();
            })
            .await;
        Self { connection }
    }

    pub(crate) async fn set(&self, key: &str, value: &str) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub(crate) async fn delete(&self, key: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::service::database::MsgDB;
    use lib::entity::Msg;

    #[tokio::test]
    async fn test() {
        let msg_db = MsgDB::new().await;
        let msg = Msg::text(1, 2, 3, "hello");
        msg_db.insert(&msg).await;
    }
}
