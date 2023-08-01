use std::net::ToSocketAddrs;

use async_trait::async_trait;
use lib::entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType};
use lib::net::{InnerStates, InnerStatesValue};
use lib::Result;
use lib_net_tokio::net::server::ReqwestCaller;
use lib_net_tokio::net::ReqwestHandler;
use tracing::info;

use crate::{cluster::ClusterCallerMap, config::config};
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
            .get_parameter::<ClusterCallerMap>()
            .unwrap();
        let cluster_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionSet>()
            .unwrap();
        let client_caller = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ReqwestCaller>()
            .unwrap();

        let server_info = ServerInfo::from(req.payload());
        info!("cluster server {} connected", server_info.id);
        cluster_set.insert(
            server_info
                .cluster_address
                .unwrap()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
        );
        cluster_map.insert(server_info.id, client_caller.clone());
        states.insert(
            "node_id".to_owned(),
            InnerStatesValue::Num(server_info.id as u64),
        );

        let res_server_info = ServerInfo {
            id: my_id(),
            service_address: config().server.service_address.clone(),
            cluster_address: Some(config().server.cluster_address.clone()),
            connection_id: 0,
            status: ServerStatus::Normal,
            typ: ServerType::SchedulerCluster,
            load: None,
        };
        Ok(ReqwestMsg::with_resource_id_payload(
            ReqwestResourceID::NodeAuth,
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
            .get_parameter::<ClusterCallerMap>()
            .unwrap();
        let cluster_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionSet>()
            .unwrap();
        let client_caller = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ReqwestCaller>()
            .unwrap();

        let res_server_info = ServerInfo::from(req.payload());
        cluster_set.insert(
            res_server_info
                .cluster_address
                .unwrap()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
        );
        cluster_map.insert(res_server_info.id, client_caller.clone());
        states.insert(
            "node_id".to_owned(),
            InnerStatesValue::Num(res_server_info.id as u64),
        );

        Ok(ReqwestMsg::default())
    }
}
