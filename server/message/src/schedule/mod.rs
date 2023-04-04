mod client;
mod handler;

use lib::{Result, net::{MsgMpmcSender, MsgMpscSender}};

pub(crate) static mut SCHEDULER_SENDER: Option<MsgMpmcSender> = None;

pub(crate) async fn start(io_task_sender: MsgMpscSender) -> Result<()> {
    client::Client::run(io_task_sender).await?;
    Ok(())
}
