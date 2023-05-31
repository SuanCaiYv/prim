use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::{server::ReqwestCaller, InnerStates, InnerStatesValue, ReqwestHandler},
    Result,
};
use tracing::info;

use crate::{cluster::ClusterCallerMap, config::CONFIG};
use crate::{cluster::ClusterConnectionSet, util::my_id};

pub(crate) struct ServerAuth {}

#[async_trait]
impl ReqwestHandler for ServerAuth {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterCallerMap>()?;
        let cluster_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionSet>()?;
        let client_caller = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ReqwestCaller>()
            .unwrap();

        let server_info = ServerInfo::from(req.payload());
        info!("cluster server {} connected", server_info.id);
        cluster_set.insert(server_info.cluster_address.unwrap());
        cluster_map.insert(server_info.id, client_caller.clone());
        states.insert(
            "node_id".to_owned(),
            InnerStatesValue::Num(server_info.id as u64),
        );

        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let mut cluster_address = CONFIG.server.cluster_address;
        cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
        let res_server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Normal,
            typ: ServerType::SchedulerCluster,
            load: None,
        };
        Ok(ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::NodeAuth.value(),
            &res_server_info.to_bytes(),
        ))
    }
}

pub(crate) struct ClientAuth {}

#[async_trait]
impl ReqwestHandler for ClientAuth {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterCallerMap>()?;
        let cluster_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionSet>()?;
        let client_caller = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ReqwestCaller>()
            .unwrap();

        let res_server_info = ServerInfo::from(req.payload());
        cluster_set.insert(res_server_info.cluster_address.unwrap());
        cluster_map.insert(res_server_info.id, client_caller.clone());
        states.insert(
            "node_id".to_owned(),
            InnerStatesValue::Num(res_server_info.id as u64),
        );

        Ok(ReqwestMsg::default())
    }
}
