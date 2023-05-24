use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    error::HandlerError,
    net::{server::Handler, InnerStates, MsgSender},
    Result,
};
use tracing::info;

use crate::util::my_id;
use crate::{cluster::ClusterConnectionMap, config::CONFIG};

pub(crate) struct ServerAuth {}

#[async_trait]
impl Handler for ServerAuth {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let cluster_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionMap>()?;
        let sender = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MsgSender>()
            .unwrap();
        let server_info = ServerInfo::from(msg.payload());
        info!("cluster server {} connected", server_info.id);
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
            typ: ServerType::MessageCluster,
            load: None,
        };
        let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
        res_msg.set_type(Type::Auth);
        res_msg.set_sender(my_id() as u64);
        res_msg.set_receiver(server_info.id as u64);
        cluster_map.insert(server_info.id, sender.clone());
        Ok(res_msg)
    }
}

pub(crate) struct ClientAuth {}

#[async_trait]
impl Handler for ClientAuth {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let authed = inner_states.get("authed");
        if authed.is_some() && authed.unwrap().is_bool() && authed.unwrap().as_bool().unwrap() {
            return Ok(msg.generate_ack(my_id(), msg.timestamp()));
        }
        let cluster_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionMap>()?;
        let sender = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MsgSender>()
            .unwrap();
        let res_server_info = ServerInfo::from(msg.payload());
        cluster_map.insert(res_server_info.id, sender.clone());
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}
