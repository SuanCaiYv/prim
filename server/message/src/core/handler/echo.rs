use anyhow::anyhow;
use tonic::async_trait;

use crate::core::{Handler, HandlerParameters};
use crate::entity::{Msg, Type};
use crate::error;
use crate::util::timestamp;

use super::super::Result;

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn handle_function(
        &self,
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = msg.clone();
            res.update_receiver(msg.receiver());
            res.update_sender(0);
            res.update_timestamp(timestamp());
            Ok(res)
        } else {
            let v = super::try_send_to_peer_directly(msg, parameters).await;
            Ok(msg.generate_ack(msg.timestamp()))
        }
    }
}

// unsafe impl Send for Echo {}
