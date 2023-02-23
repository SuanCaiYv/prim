use lib::entity::Msg;
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
    extension   BLOB,
    payload     BLOB,
    status      INTEGER
)";

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

    pub(crate) async fn insert(&self, msg: &Msg) {
        self.connection
            .call(move |conn| {
                let mut stmt = conn
                    .execute(
                        "INSERT INTO msg (
                            sender,
                            receiver,
                            \"timestamp\",
                            seq_num,
                            type,
                            version,
                            extension,
                            payload,
                            status
                        ) VALUES (
                            ?1,
                            ?2,
                            ?3,
                            ?4,
                            ?5,
                            ?6,
                            ?7,
                            ?8,
                            ?9
                        )", (params![
                            msg.sender(),
                            msg.receiver(),
                            msg.timestamp(),
                            msg.seq_num(),
                            msg.typ(),
                            msg.version(),
                            msg.extension(),
                            msg.payload(),
                            1
                        ])
                    );
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use lib::entity::Msg;
    use crate::service::database::MsgDB;

    #[tokio::test]
    async fn test() {
        let msg_db = MsgDB::new().await;
        let msg = Msg::text(1, 2, 3, "hello");
        msg_db.insert(&msg).await;
    }
}
