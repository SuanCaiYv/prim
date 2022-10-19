use std::time::SystemTime;
use common::entity::Type;

#[derive(sqlx::FromRow)]
#[table_name = "message"]
pub(crate) struct Message {
    pub(crate) id: u64,
    pub(crate) sender: u64,
    pub(crate) receiver: u64,
    pub(crate) timestamp: SystemTime,
    pub(crate) seq_num: u64,
    pub(crate) typ: Type,
    pub(crate) version: u16,
    pub(crate) extension: String,
    pub(crate) payload: String,
    pub(crate) status: u16,
}