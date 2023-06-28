use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo, ServerStatus},
    net::{InnerStates, InnerStatesValue, ReqwestHandler},
    Result,
};
use lib_net_tokio::net::server::ReqwestCaller;

use crate::{
    config::CONFIG,
    service::{ClientCallerMap},
    util::my_id,
};

pub(crate) struct ServerAuth {}

#[async_trait]
impl ReqwestHandler for ServerAuth {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientCallerMap>()?;
        let client_caller = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ReqwestCaller>()?;

        let server_info = ServerInfo::from(req.payload());
        client_map.insert(server_info.id, client_caller.clone());
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
            typ: server_info.typ,
            load: None,
        };
        let res_msg =
            ReqwestMsg::with_resource_id_payload(req.resource_id(), &res_server_info.to_bytes());
        Ok(res_msg)
    }
}
