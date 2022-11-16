use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;

use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};

use crate::service::ClientSenderTimeoutReceiverMap;

pub(super) struct NodeRegister {}

#[async_trait]
impl Handler for NodeRegister {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::NodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientSenderTimeoutReceiverMap>();
        if let Err(_) = client_map {
            return Err(anyhow!("client map not found"));
        }
        let client_map = client_map.unwrap();
        let notify_msg = (*msg).clone();
        let notify_msg = Arc::new(notify_msg);
        for entry in client_map.0.iter() {
            if *entry.key() as u64 == msg.sender() {
                continue;
            }
            entry.value().send(notify_msg.clone()).await?;
        }
        Ok(msg.generate_ack(msg.timestamp()))
    }
}

pub(super) struct NodeUnregister {}

#[async_trait]
impl Handler for NodeUnregister {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::NodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientSenderTimeoutReceiverMap>();
        if let Err(_) = client_map {
            return Err(anyhow!("client map not found"));
        }
        let client_map = client_map.unwrap();
        let notify_msg = (*msg).clone();
        let notify_msg = Arc::new(notify_msg);
        for entry in client_map.0.iter() {
            if *entry.key() as u64 == msg.sender() {
                continue;
            }
            entry.value().send(notify_msg.clone()).await?;
        }
        Ok(Msg::noop())
    }
}
