use anyhow::anyhow;
use tonic::async_trait;
use tracing::log::debug;

use crate::core::{Handler, HandlerParameters};
use crate::entity::msg;
use crate::error;
use crate::util::timestamp;

use super::super::Result;

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn handle_function(
        &self,
        msg: &mut msg::Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<msg::Msg> {
        if msg::Type::Echo != msg.head.typ {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        if msg.head.receiver == 0 {
            let mut res = msg.duplicate();
            res.head.receiver = msg.head.receiver;
            res.head.sender = 0;
            res.head.timestamp = timestamp();
            debug!("echo: {}", msg);
            Ok(res)
        } else {
            super::send_to_peer(msg, parameters).await?;
            Ok(msg.generate_ack(msg.head.timestamp))
        }
    }
}

// unsafe impl Send for Echo {}
