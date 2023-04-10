use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, Type},
    error::HandlerError,
    net::{
        server::{Handler, HandlerParameters, InnerStates},
        MsgSender,
    },
    Result, MESSAGE_NODE_ID_BEGINNING, SCHEDULER_NODE_ID_BEGINNING,
};

use crate::{
    config::CONFIG,
    service::{
        ClientConnectionMap, MessageNodeSet, SchedulerNodeSet, ServerInfoMap,
    },
    util::my_id,
};

pub(crate) struct ServerAuth {}

#[async_trait]
impl Handler for ServerAuth {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        _inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?;
        let server_info_map = parameters
            .generic_parameters
            .get_parameter::<ServerInfoMap>()?;
        let message_node_set = parameters
            .generic_parameters
            .get_parameter::<MessageNodeSet>()?;
        let scheduler_node_set = parameters
            .generic_parameters
            .get_parameter::<SchedulerNodeSet>()?;
        let sender = parameters.generic_parameters.get_parameter::<MsgSender>()?;
        let server_info = ServerInfo::from(msg.payload());
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
        let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
        res_msg.set_type(Type::Auth);
        res_msg.set_sender(my_id() as u64);
        res_msg.set_receiver(server_info.id as u64);
        client_map.insert(server_info.id, sender.clone());
        server_info_map.insert(server_info.id, server_info);
        if server_info.id >= MESSAGE_NODE_ID_BEGINNING
            && server_info.id < SCHEDULER_NODE_ID_BEGINNING
        {
            message_node_set.insert(server_info.id);
        } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING
        {
            scheduler_node_set.insert(server_info.id);
        }
        Ok(res_msg)
    }
}
