use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo, ServerStatus},
    net::{InnerStates, ReqwestHandler},
    Result, MESSAGE_NODE_ID_BEGINNING, SCHEDULER_NODE_ID_BEGINNING,
};

use crate::{
    config::CONFIG,
    service::{ClientConnectionMap, MessageNodeSet, SchedulerNodeSet, ServerInfoMap},
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
            .get_parameter::<ClientConnectionMap>()?;
        let server_info_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ServerInfoMap>()?;
        let message_node_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MessageNodeSet>()?;
        let scheduler_node_set = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<SchedulerNodeSet>()?;
        let server_info = ServerInfo::from(req.payload());
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
        server_info_map.insert(server_info.id, server_info);
        if server_info.id >= MESSAGE_NODE_ID_BEGINNING
            && server_info.id < SCHEDULER_NODE_ID_BEGINNING
        {
            message_node_set.insert(server_info.id);
        } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING {
            scheduler_node_set.insert(server_info.id);
        }
        Ok(res_msg)
    }
}
