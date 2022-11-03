use crate::inner::StatusMap;
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
        let status_map = generic_map.get_parameter_mut::<StatusMap>();
        if status_map.is_err() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "status map not found".to_string()
            )));
        }
        let status_map = &status_map.unwrap().0;
        let node_info = NodeInfo::from(msg.payload());
        status_map.insert(msg.sender() as u32, node_info.clone());
        let mut register_msg = Msg::raw_payload(&node_info.to_bytes());
        register_msg.set_type(Type::NodeRegister);
        parameters.inner_sender.send(Arc::new(register_msg)).await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
