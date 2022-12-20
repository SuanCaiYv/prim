mod client;
mod handler;

use lib::{Result, net::{OuterSender, InnerSender}};

pub(crate) static mut SCHEDULER_SENDER: Option<OuterSender> = None;

pub(crate) async fn start(io_task_sender: InnerSender) -> Result<()> {
    client::Client::run(io_task_sender).await?;
    Ok(())
}
