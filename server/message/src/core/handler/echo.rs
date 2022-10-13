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
        let mut res = msg.duplicate();
        res.head.receiver = msg.head.receiver;
        res.head.sender = 0;
        res.head.timestamp = timestamp();
        debug!("echo: {}", msg);
        Ok(res)
    }
}

// unsafe impl Send for Echo {}
