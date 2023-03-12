use std::sync::Arc;

use tokio::sync::OnceCell;

use self::database::{MsgDB, KVDB};

pub(crate) mod database;

pub(crate) static MSG_DB: OnceCell<Arc<MsgDB>> = OnceCell::const_new();
pub(crate) static KV_DB: OnceCell<Arc<KVDB>> = OnceCell::const_new();

pub(super) async fn get_msg_ops() -> Arc<MsgDB> {
    MSG_DB
        .get_or_init(|| async {
            let db = MsgDB::new().await;
            Arc::new(db)
        })
        .await
        .clone()
}

pub(super) async fn get_kv_ops() -> Arc<KVDB> {
    KV_DB
        .get_or_init(|| async {
            let db = KVDB::new().await;
            Arc::new(db)
        })
        .await
        .clone()
}