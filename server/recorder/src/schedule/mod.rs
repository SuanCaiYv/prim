mod client;
mod handler;

use lib::{Result, net::OuterSender};

pub(crate) static mut SCHEDULER_SENDER: Option<OuterSender> = None;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await?;
    Ok(())
}
