use lib::{Result, net::{OuterSender, OuterReceiver}};

pub(super) async fn handler_func(_io_channel: (OuterSender, OuterReceiver), _timeout_receiver: OuterReceiver) -> Result<()> {
    Ok(())
}