use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo},
    net::InnerStates,
    Result,
};
use lib_net_tokio::net::ReqwestHandler;

use crate::service::ClientCallerMap;

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(
        &self,
        req: &mut ReqwestMsg,
        inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        let client_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()
            .unwrap();

        let server_info = ServerInfo::from(req.payload());
        let mut bytes = vec![1u8];
        bytes.extend_from_slice(&server_info.to_bytes());
        let notify_msg =
            ReqwestMsg::with_resource_id_payload(ReqwestResourceID::MessageNodeRegister, &bytes);
        for entry in client_map.0.iter() {
            if *entry.key() == server_info.id {
                continue;
            }
            entry.value().call(notify_msg.clone()).await?;
        }
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl ReqwestHandler for NodeUnregister {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()
            .unwrap();
        let server_info = ServerInfo::from(req.payload());
        let notify_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::MessageNodeUnregister,
            &server_info.to_bytes(),
        );
        for entry in client_map.0.iter() {
            if *entry.key() == server_info.id {
                continue;
            }
            entry.value().call(notify_msg.clone()).await?;
        }
        Ok(ReqwestMsg::default())
    }
}
