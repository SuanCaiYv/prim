use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;

use lib::{
    entity::{Msg, Type, ReqwestMsg},
    error::HandlerError,
    net::{server::{Handler}, ReqwestHandler, InnerStates},
    Result,
};

use crate::service::ClientConnectionMap;
use crate::util::my_id;

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(
        &self,
        msg: &mut ReqwestMsg,
        inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        if msg.typ() != Type::MessageNodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = inner_states.get("generic_map").unwrap().as_generic_parameter_map().unwrap().get_parameter::<ClientConnectionMap>()?;
        let mut notify_msg = Msg::from_payload_extension(msg.payload(), b"true");
        notify_msg.set_type(Type::MessageNodeRegister);
        notify_msg.set_sender(msg.sender());
        let notify_msg = Arc::new(notify_msg);
        for entry in client_map.iter() {
            entry.value().send(notify_msg.clone()).await?;
        }
        let client_timestamp = inner_states
            .get("client_timestamp")
            .unwrap()
            .as_num()
            .unwrap();
        Ok(msg.generate_ack(my_id(), client_timestamp))
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl Handler for NodeUnregister {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        _inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::MessageNodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>();
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
