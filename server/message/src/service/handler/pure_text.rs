use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::Msg,
    error::HandlerError,
    net::{InnerStates, InnerStatesValue},
    Result,
};
use lib_net_tokio::net::Handler;
use tracing::{debug, error};

use crate::{
    cluster::ClusterConnectionMap,
    rpc::{get_rpc_client, node::RpcClient},
    service::handler::{IOTaskMsg::Direct, IOTaskSender},
    service::ClientConnectionMap,
    util::my_id,
};

use super::{is_group_msg, push_group_msg};

pub(crate) struct PureText;

#[async_trait]
impl Handler for PureText {
    async fn run(&self, msg: &mut Arc<Msg>, states: &mut InnerStates) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value < 32 || type_value >= 64 {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let receiver = msg.receiver();
        let node_id = msg.node_id();
        if is_group_msg(receiver) {
            if states.get("group_node_list_map").is_none() {
                states.insert(
                    "group_node_list_map".to_owned(),
                    InnerStatesValue::NumListMap(AHashMap::new()),
                );
            }
            if states
                .get("generic_map")
                .unwrap()
                .as_generic_parameter_map()
                .unwrap()
                .get_parameter::<RpcClient>()
                .is_none()
            {
                let rpc_client = get_rpc_client().await;
                states
                    .get_mut("generic_map")
                    .unwrap()
                    .as_mut_generic_parameter_map()
                    .unwrap()
                    .put_parameter(rpc_client);
            }
            if states
                .get("group_node_list_map")
                .unwrap()
                .as_num_list_map()
                .unwrap()
                .get(&receiver)
                .is_none()
            {
                let rpc_client = states
                    .get_mut("generic_map")
                    .unwrap()
                    .as_mut_generic_parameter_map()
                    .unwrap()
                    .get_parameter_mut::<RpcClient>()
                    .unwrap();
                let list = rpc_client
                    .call_all_group_node_list(receiver)
                    .await?
                    .into_iter()
                    .map(|v| v as u64)
                    .collect::<Vec<u64>>();
                states
                    .get_mut("group_node_list_map")
                    .unwrap()
                    .as_mut_num_list_map()
                    .unwrap()
                    .insert(receiver, list.clone());
            }
        }
        let client_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientConnectionMap>()
            .unwrap();
        let cluster_map = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClusterConnectionMap>()
            .unwrap();
        let io_task_sender = states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<IOTaskSender>()
            .unwrap();
        if is_group_msg(receiver) {
            let node_list = states
                .get("group_node_list_map")
                .unwrap()
                .as_num_list_map()
                .unwrap()
                .get(&receiver)
                .unwrap()
                .into_iter()
                .map(|v| *v as u32)
                .collect::<Vec<u32>>();
            for node_id in node_list {
                if node_id == my_id() {
                    push_group_msg(msg.clone(), false).await?;
                    continue;
                }
                match cluster_map.get(&node_id) {
                    Some(sender) => {
                        if let Err(e) = sender.send(msg.clone()).await {
                            // this one and the blow error should be handle by scheduler to
                            // check where remote node is still alive.
                            // and whether new connection need to be established.
                            error!("send to cluster[{}] error: {}", node_id, e);
                            return Err(anyhow!(HandlerError::IO(
                                "server cluster crashed!".to_string()
                            )));
                        }
                    }
                    None => {
                        // same as above.
                        // todo!() should be handled by scheduler!!!
                        error!("cluster[{}] offline!", node_id);
                        return Err(anyhow!(HandlerError::IO(
                            "server cluster crashed!".to_string()
                        )));
                    }
                }
            }
        } else {
            io_task_sender.send(Direct(msg.clone())).await?;
            if node_id == my_id() {
                match client_map.get(&receiver) {
                    Some(client_sender) => {
                        if let Err(e) = client_sender.send(msg.clone()).await {
                            error!("send to client[{}] error: {}", receiver, e);
                        }
                    }
                    None => {
                        debug!("receiver {} not found", receiver);
                    }
                }
            } else {
                match cluster_map.get(&node_id) {
                    Some(sender) => {
                        if let Err(e) = sender.send(msg.clone()).await {
                            // this one and the blow error should be handle by scheduler to
                            // check where remote node is still alive.
                            // and whether new connection need to be established.
                            error!("send to cluster[{}] error: {}", node_id, e);
                            return Err(anyhow!(HandlerError::IO(
                                "server cluster crashed!".to_string()
                            )));
                        }
                    }
                    None => {
                        // same as above.
                        // todo!() should be handled by scheduler!!!
                        error!("cluster[{}] offline!", node_id);
                        return Err(anyhow!(HandlerError::IO(
                            "server cluster crashed!".to_string()
                        )));
                    }
                }
            }
        }
        let client_timestamp = states.get("client_timestamp").unwrap().as_num().unwrap();
        Ok(msg.generate_ack(my_id(), client_timestamp))
    }
}
