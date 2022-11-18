use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use common::entity::{Msg, Type};
use common::error;
use common::net::server::{Handler, HandlerParameters};

pub(crate) struct Balancer;

#[async_trait]
impl Handler for Balancer {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        match msg.typ() {
            Type::NodeRegister | Type::NodeUnregister => {
                parameters.io_handler_sender.send(msg.clone()).await?;
            }
            Type::UserNodeMapChange => {}
            _ => {
                return Err(anyhow!(error::HandlerError::NotMine));
            }
        };
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
