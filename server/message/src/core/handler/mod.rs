use crate::core::HandlerParameters;
use crate::core::Result;
use crate::entity::msg;

pub(super) mod auth;
pub(super) mod echo;
pub(super) mod text;

pub(self) async fn send_to_peer(msg: &msg::Msg, parameters: &mut HandlerParameters) -> Result<()> {
    let send = parameters.connection_map.get(&msg.head.receiver);
    if let Some(send) = send {
        let res = send.send(msg.clone()).await;
        if let Err(_) = res {
            // when connection closed, the receiver's `close()` will be invoked.
            parameters.connection_map.remove(&msg.head.receiver);
        }
    }
    Ok(())
}
