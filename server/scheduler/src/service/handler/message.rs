use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;

use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};

use crate::util::my_id;
use crate::{
    cluster::ClusterConnectionMap,
    service::{ClientConnectionMap, ServerInfoMap},
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl Handler<()> for NodeRegister {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters, _inner_states: &mut InnerStates<()>) -> Result<Msg> {
        if msg.typ() != Type::MessageNodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>();
        let server_info_map = parameters
            .generic_parameters
            .get_parameter::<ServerInfoMap>();
        let cluster_map = parameters
            .generic_parameters
            .get_parameter::<ClusterConnectionMap>();
        if let Err(_) = client_map {
            return Err(anyhow!("client map not found"));
        }
        if let Err(_) = server_info_map {
            return Err(anyhow!("server info map not found"));
        }
        if let Err(_) = cluster_map {
            return Err(anyhow!("cluster map not found"));
        }
        let client_map = &client_map.unwrap().0;
        let server_info_map = &server_info_map.unwrap().0;
        let cluster_map = &cluster_map.unwrap().0;
        let self_sender = client_map.get(&(msg.sender() as u32));
        if let None = self_sender {
            return Err(anyhow!("self sender not found"));
        }
        let self_sender = self_sender.unwrap();
        let mut notify_msg = Msg::from_payload_extension(msg.payload(), b"true");
        notify_msg.set_type(Type::MessageNodeRegister);
        notify_msg.set_sender(msg.sender());
        let notify_msg = Arc::new(notify_msg);
        for entry in client_map.iter() {
            if *entry.key() as u64 == msg.sender() {
                continue;
            }
            entry.value().send(notify_msg.clone()).await?;
            let server_info = server_info_map.get(entry.key());
            if let Some(server_info) = server_info {
                let mut res_notify_msg =
                    Msg::from_payload_extension(&server_info.to_bytes(), b"false");
                res_notify_msg.set_type(Type::MessageNodeRegister);
                res_notify_msg.set_receiver(msg.sender());
                res_notify_msg.set_node_id(msg.sender() as u32);
                self_sender.send(Arc::new(res_notify_msg)).await?;
            }
        }
        for entry in cluster_map.iter() {
            entry.value().send(msg.clone()).await?;
        }
        Ok(msg.generate_ack(my_id()))
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl Handler<()> for NodeUnregister {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters, _inner_states: &mut InnerStates<()>) -> Result<Msg> {
        if msg.typ() != Type::MessageNodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>();
        let server_info_map = parameters
            .generic_parameters
            .get_parameter::<ServerInfoMap>();
        let cluster_map = parameters
            .generic_parameters
            .get_parameter::<ClusterConnectionMap>();
        if let Err(_) = client_map {
            return Err(anyhow!("client map not found"));
        }
        if let Err(_) = server_info_map {
            return Err(anyhow!("server info map not found"));
        }
        if let Err(_) = cluster_map {
            return Err(anyhow!("cluster map not found"));
        }
        let client_map = &client_map.unwrap().0;
        let cluster_map = &cluster_map.unwrap().0;
        let mut notify_msg = Msg::from_payload_extension(msg.payload(), b"true");
        notify_msg.set_type(Type::MessageNodeUnregister);
        notify_msg.set_sender(msg.sender());
        let notify_msg = Arc::new(notify_msg);
        for entry in client_map.iter() {
            if *entry.key() as u64 == msg.sender() {
                continue;
            }
            entry.value().send(notify_msg.clone()).await?;
        }
        for entry in cluster_map.iter() {
            entry.value().send(msg.clone()).await?;
        }
        Ok(Msg::noop())
    }
}
