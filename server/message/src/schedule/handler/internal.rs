use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;

use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};

pub(super) struct NodeRegister {}

#[async_trait]
impl Handler for NodeRegister {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::NodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        crate::cluster::node_online(msg.clone()).await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}

pub(super) struct NodeUnregister {}

#[async_trait]
impl Handler for NodeUnregister {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::NodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        crate::cluster::node_offline(msg.clone()).await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
