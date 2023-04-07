mod client;
mod handler;

use lib::{net::MsgMpmcSender, Result};

use crate::service::handler::IOTaskSender;

pub(crate) static mut SCHEDULER_SENDER: Option<MsgMpmcSender> = None;

pub(crate) async fn start(io_task_sender: IOTaskSender) -> Result<()> {
    client::Client::run(io_task_sender).await?;
    Ok(())
}
