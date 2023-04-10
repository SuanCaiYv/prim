mod client;
mod handler;

use lib::{net::MsgSender, Result};

pub(crate) static mut SCHEDULER_SENDER: Option<MsgSender> = None;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await?;
    Ok(())
}
