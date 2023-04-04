use lib::{net::MsgMpmcSender, Result};

mod client;

pub(self) static mut RECORDER_SENDER: Option<MsgMpmcSender> = None;

#[inline]
pub(crate) fn recorder_sender() -> &'static mut MsgMpmcSender {
    unsafe { RECORDER_SENDER.as_mut().unwrap() }
}

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await
}