use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo},
    net::InnerStates,
    Result,
};
use lib_net_tokio::net::ReqwestHandler;

use crate::{
    cluster::ClusterCallerMap,
    service::{ClientCallerMap, SeqnumNodeSet, ServerInfoMap},
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let _client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()
            .unwrap();
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()
            .unwrap();
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterCallerMap>()
            .unwrap();
        let _seqnum_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<SeqnumNodeSet>()
            .unwrap();

        let server_info = ServerInfo::from(req.payload());
        server_info_map.insert(server_info.id, server_info);

        // code blow used for notify other seqnum nodes.

        // let self_sender = client_map.get(server_info.id);
        // if self_sender.is_none() {
        //     return Err(anyhow!("self sender not found"));
        // }
        // let self_sender = self_sender.unwrap();
        // let mut bytes = vec![1u8];
        // bytes.extend_from_slice(&server_info.to_bytes());
        // let notify_msg =
        //     ReqwestMsg::with_resource_id_payload(ReqwestResourceID::MessageNodeRegister, &bytes);
        // for entry in seqnum_set.0.iter() {
        //     if *entry.key() == server_info.id {
        //         continue;
        //     }
        //     match client_map.get(*entry.key()) {
        //         Some(caller) => caller.call(notify_msg.clone()).await?,
        //         None => continue,
        //     };
        //     let peer_info = server_info_map.get(*entry.key());
        //     if let Some(peer_info) = peer_info {
        //         let peer_notify_msg = ReqwestMsg::with_resource_id_payload(
        //             ReqwestResourceID::MessageNodeRegister,
        //             &peer_info.to_bytes(),
        //         );
        //         self_sender.call(peer_notify_msg).await?;
        //     }
        // }
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
            .get_parameter::<ClientCallerMap>()
            .unwrap();
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()
            .unwrap();
        let _cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterCallerMap>()
            .unwrap();
        let seqnum_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<SeqnumNodeSet>()
            .unwrap();
        // let notify_msg = ReqwestMsg::with_resource_id_payload(
        //     ReqwestResourceID::MessageNodeUnregister,
        //     &server_info.to_bytes(),
        // );
        // for entry in seqnum_set.0.iter() {
        //     if *entry.key() == server_info.id {
        //         continue;
        //     }
        //     match client_map.get(*entry.key()) {
        //         Some(caller) => caller.call(notify_msg.clone()).await?,
        //         None => continue,
        //     };
        // }
        // for entry in cluster_map.0.iter() {
        //     entry.value().call(req.clone()).await?;
        // }
        client_map.remove(server_info.id as u32);
        server_info_map.remove(server_info.id as u32);
        seqnum_set.remove(server_info.id as u32);
        Ok(ReqwestMsg::default())
    }
}
