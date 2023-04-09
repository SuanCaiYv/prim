use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;

use crate::{service::server::InnerValue, util::my_id};
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters, InnerStates},
    Result,
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl Handler<InnerValue> for NodeRegister {
    async fn run(
        &self,
        msg: Arc<Msg>,
        _parameters: &mut HandlerParameters,
        _inner_states: &mut InnerStates<InnerValue>,
    ) -> Result<Msg> {
        if msg.typ() != Type::MessageNodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        crate::cluster::node_online(msg.clone()).await?;
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl Handler<InnerValue> for NodeUnregister {
    async fn run(
        &self,
        msg: Arc<Msg>,
        _parameters: &mut HandlerParameters,
        _inner_states: &mut InnerStates<InnerValue>,
    ) -> Result<Msg> {
        if msg.typ() != Type::MessageNodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        crate::cluster::node_offline(msg.clone()).await?;
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}
