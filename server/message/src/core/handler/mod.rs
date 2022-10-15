use crate::core::HandlerParameters;
use crate::core::Result;
use crate::entity::Msg;
use tracing::log::debug;

pub(super) mod auth;
pub(super) mod echo;
pub(super) mod text;

pub(self) async fn try_send_to_peer_directly(
    msg: &Msg,
    parameters: &mut HandlerParameters,
) -> Result<()> {
    let mut should_remove = false;
    // reduce the reference scope to promote release of the lock.
    {
        let send = parameters.connection_map.get(&msg.receiver());
        if let Some(send) = send {
            let res = send.send(msg.clone()).await;
            if let Err(_) = res {
                should_remove = true;
            }
        }
    }
    if should_remove {
        // when connection closed, the receiver's `close()` will be invoked.
        parameters.connection_map.remove(&msg.receiver());
        debug!("user: {} may be offline", msg.receiver());
    }
    Ok(())
}
