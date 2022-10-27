use crate::inner::{ConnectionId, StatusMap};
use anyhow::anyhow;
use async_trait::async_trait;
use common::{
    entity::{Msg, NodeInfo, Type},
    error::{CrashError, HandlerError},
    net::server::{Handler, HandlerParameters},
};
use std::sync::Arc;

pub(crate) struct Register;

#[async_trait]
impl Handler for Register {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        if msg.typ() != Type::NodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let generic_map = &mut parameters.generic_parameters;
        let connection_id = generic_map.get_parameter_mut::<ConnectionId>();
        if connection_id.is_err() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "connection id not found".to_string()
            )));
        }
        let connection_id = connection_id.unwrap().0;
        let status_map = generic_map.get_parameter_mut::<StatusMap>();
        if status_map.is_err() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "status map not found".to_string()
            )));
        }
        let status_map = status_map.unwrap().0;
        let node_info = NodeInfo::from(msg.payload());
        node_info.connection_id = connection_id;
        status_map.insert(connection_id, node_info.clone());
        let mut register_msg = Msg::raw_payload(&node_info.to_bytes());
        register_msg.update_type(Type::NodeRegister);
        parameters
            .inner_channel
            .0
            .send(Arc::new(register_msg))
            .await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
