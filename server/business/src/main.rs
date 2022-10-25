mod config;
mod entity;
mod sql;

use crate::entity::msg::Message;

use common::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let msg = Message::get(8).await?;
    println!("{}", msg);
    Ok(())
}
