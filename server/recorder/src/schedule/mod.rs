mod client;
mod handler;

use lib::{Result, net::MsgMpmcSender};

pub(crate) static mut SCHEDULER_SENDER: Option<MsgMpmcSender> = None;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await?;
    Ok(())
}
