mod config;
mod entity;
mod sql;

use crate::entity::msg::Message;
use crate::sql::SQL_POOL;

use common::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = sql::sql_connection_pool().await?;
    unsafe {
        SQL_POOL = Some(pool);
    }
    let mut msg = Message::get(8).await?;
    println!("{}", msg);
    Ok(())
}
