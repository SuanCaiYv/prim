use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{entity::Msg, error::HandlerError, net::InnerStates, Result};
use lib_net_tokio::net::Handler;
use tracing::{debug, error};

use crate::{
    cluster::ClusterConnectionMap,
    service::handler::{IOTaskMsg::Direct, IOTaskSender},
    service::ClientConnectionMap,
    util::my_id,
};

use super::{is_group_msg, push_group_msg};

pub(crate) struct PureText;

#[async_trait]
impl Handler for PureText {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value < 32 || type_value >= 64 {
            return Err(anyhow!(HandlerError::NotMine));
        }
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
                        if let Err(e) = client_sender.send(msg.clone()).await {
                            error!("send to client[{}] error: {}", receiver, e);
                        }
                    }
                    None => {
                        debug!("receiver {} not found", receiver);
                    }
                }
                // each node only records self's msg.
                // group message will be recorded by group task.
                io_task_sender.send(Direct(msg.clone())).await?;
            }
            let client_timestamp = inner_states
                .get("client_timestamp")
                .unwrap()
                .as_num()
                .unwrap();
            Ok(msg.generate_ack(my_id(), client_timestamp))
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
            Ok(Msg::noop())
        }
    }
}
