use crate::core::Handler;
use anyhow::anyhow;
use common::entity::{Msg, Type};
use common::net::HandlerParameters;
use common::util::timestamp;
use std::sync::Arc;
use tonic::async_trait;

use crate::error;

use common::Result;

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = (*msg).clone();
            res.update_receiver(msg.receiver());
            res.update_sender(0);
            res.update_timestamp(timestamp());
            Ok(res)
        } else {
            Ok(msg.generate_ack(msg.timestamp()))
        }
    }
}

// unsafe impl Send for Echo {}
