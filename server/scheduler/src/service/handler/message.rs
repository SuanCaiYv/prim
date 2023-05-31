use anyhow::anyhow;
use async_trait::async_trait;

use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo},
    net::{InnerStates, ReqwestHandler},
    Result,
};

use crate::service::{ServerInfoMap, ClientCallerMap};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()?;
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()?;
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()?;

        let server_info = ServerInfo::from(req.payload());
        let self_sender = client_map.get(server_info.id);
        if self_sender.is_none() {
            return Err(anyhow!("self sender not found"));
        }
        let self_sender = self_sender.unwrap();
        let mut bytes = vec![1u8];
        bytes.extend_from_slice(&server_info.to_bytes());
        let notify_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::MessageNodeRegister.value(),
            &bytes,
        );
        for entry in client_map.0.iter() {
            if *entry.key() == server_info.id {
                continue;
            }
            entry.value().call(notify_msg.clone()).await?;
            let peer_info = server_info_map.get(entry.key());
            if let Some(peer_info) = peer_info {
                let mut res_notify_msg = ReqwestMsg::with_resource_id_payload(
                    ReqwestResourceID::MessageNodeRegister.value(),
                    &peer_info.to_bytes(),
                );
                self_sender.call(res_notify_msg).await?;
            }
        }
        // todo
        for entry in cluster_map.0.iter() {
            entry.value().call(req.clone()).await?;
        }
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl ReqwestHandler for NodeUnregister {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let server_info = ServerInfo::from(req.payload());
        let client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()?;
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()?;
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()?;
        let mut notify_msg = ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::MessageNodeUnregister.value(),
            &server_info.to_bytes(),
        );
        for entry in client_map.0.iter() {
            if *entry.key() == server_info.id {
                continue;
            }
            entry.value().call(notify_msg.clone()).await?;
        }
        for entry in cluster_map.0.iter() {
            entry.value().call(req.clone()).await?;
        }
        Ok(ReqwestMsg::default())
    }
}
