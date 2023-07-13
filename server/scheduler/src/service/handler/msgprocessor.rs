use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo},
    net::InnerStates,
    Result,
};
use lib_net_tokio::net::ReqwestHandler;

use crate::{
    cluster::ClusterCallerMap,
    service::{ClientCallerMap, MsgprocessorSet, ServerInfoMap},
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
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

        let server_info = ServerInfo::from(req.payload());
        server_info_map.insert(server_info.id, server_info);
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
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterCallerMap>()
            .unwrap();
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()
            .unwrap();
        let set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MsgprocessorSet>()
            .unwrap();
        for entry in cluster_map.0.iter() {
            entry.value().call(req.clone()).await?;
        }
        client_map.remove(server_info.id as u32);
        server_info_map.remove(server_info.id as u32);
        set.remove(server_info.id as u32);
        Ok(ReqwestMsg::default())
    }
}
