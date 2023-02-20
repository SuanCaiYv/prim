use lib::{net::OuterSender, Result};

mod client;

pub(self) static mut RECORDER_SENDER: Option<OuterSender> = None;

#[inline]
pub(crate) fn recorder_sender() -> &'static mut OuterSender {
    unsafe { RECORDER_SENDER.as_mut().unwrap() }
}

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await
}