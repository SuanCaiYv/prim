mod pure_text;

use lib::{net::MsgMpscReceiver, Result};

use super::MsgSender;

pub(super) async fn handler_func(
    _sender: MsgSender,
    _receiver: MsgMpscReceiver,
    _timeout_receiver: MsgMpscReceiver,
) -> Result<()> {
    Ok(())
}
