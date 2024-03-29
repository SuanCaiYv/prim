use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{entity::Msg, error::HandlerError, net::InnerStates, Result};
use lib_net_tokio::net::Handler;
use tracing::{debug, error};

use crate::service::handler::IOTaskMsg::Direct;
use crate::service::handler::IOTaskSender;
use crate::{cluster::ClusterConnectionMap, service::ClientConnectionMap, util::my_id};

use super::{is_group_msg, push_group_msg};

pub(crate) struct ControlText;

#[async_trait]
impl Handler for ControlText {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value >= 64 && type_value < 96 {
            let client_map = inner_states
                .get("generic_map")
                .unwrap()
                .as_generic_parameter_map()
                .unwrap()
                .get_parameter::<ClientConnectionMap>()
                .unwrap();
            let cluster_map = inner_states
                .get("generic_map")
                .unwrap()
                .as_generic_parameter_map()
                .unwrap()
                .get_parameter::<ClusterConnectionMap>()
                .unwrap();
            let io_task_sender = inner_states
                .get("generic_map")
                .unwrap()
                .as_generic_parameter_map()
                .unwrap()
                .get_parameter::<IOTaskSender>()
                .unwrap();
            let receiver = msg.receiver();
            let node_id = msg.node_id();
            if node_id == my_id() {
                if is_group_msg(receiver) {
                    push_group_msg(msg.clone(), true).await?;
                } else {
                    match client_map.get(&receiver) {
                        Some(client_sender) => {
                            client_sender.send(msg.clone()).await?;
                        }
                        None => {
                            debug!("receiver {} not found", receiver);
                        }
                    }
                    io_task_sender.send(Direct(msg.clone())).await?;
                }
            } else {
                match cluster_map.get(&node_id) {
                    Some(sender) => {
                        sender.send(msg.clone()).await?;
                    }
                    None => {
                        // todo
                        error!("cluster[{}] offline!", node_id);
                    }
                }
            }
            let client_timestamp = inner_states
                .get("client_timestamp")
                .unwrap()
                .as_num()
                .unwrap();
            Ok(msg.generate_ack(my_id(), client_timestamp))
        } else {
            Err(anyhow!(HandlerError::NotMine))
        }
    }
}
