use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::Msg,
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};
use tracing::{debug, error};

use crate::service::handler::IOTaskSender;
use crate::service::{handler::IOTaskMsg::Direct, server::InnerValue};
use crate::{cluster::ClusterConnectionMap, service::ClientConnectionMap, util::my_id};

use super::{is_group_msg, push_group_msg};

pub(crate) struct PureText;

#[async_trait]
impl Handler<InnerValue> for PureText {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        _inner_state: &mut AHashMap<String, InnerValue>,
    ) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value < 32 || type_value >= 64 {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = &parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?
            .0;
        let cluster_map = &parameters
            .generic_parameters
            .get_parameter::<ClusterConnectionMap>()?
            .0;
        let io_task_sender = parameters
            .generic_parameters
            .get_parameter::<IOTaskSender>()?;
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
                io_task_sender.send(Direct(msg.clone())).await?;
            }
        } else {
            match cluster_map.get(&node_id) {
                Some(sender) => {
                    if let Err(e) = sender.send(msg.clone()).await {
                        error!("send to cluster[{}] error: {}", node_id, e);
                    }
                }
                None => {
                    // todo
                    error!("cluster[{}] offline!", node_id);
                }
            }
        }
        Ok(msg.generate_ack(my_id()))
    }
}
