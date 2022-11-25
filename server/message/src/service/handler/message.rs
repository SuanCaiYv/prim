use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters, WrapInnerSender},
    Result,
};
use tracing::{debug, error};

use crate::{cluster::ClusterSenderTimeoutReceiverMap, service::ClientConnectionMap, util::my_id};

use super::{is_group_msg, push_group_msg};

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if Type::Text != msg.typ()
            && Type::Meme != msg.typ()
            && Type::File != msg.typ()
            && Type::Image != msg.typ()
            && Type::Audio != msg.typ()
            && Type::Video != msg.typ()
        {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = &parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?
            .0;
        let cluster_map = &parameters
            .generic_parameters
            .get_parameter::<ClusterSenderTimeoutReceiverMap>()?
            .0;
        let io_task_sender = &parameters
            .generic_parameters
            .get_parameter::<WrapInnerSender>()?
            .0;
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
            }
        } else {
            match cluster_map.get(&node_id) {
                Some(cluster_sender) => {
                    cluster_sender.send(msg.clone()).await?;
                }
                None => {
                    // todo
                    error!("cluster[{}] offline!", node_id);
                }
            }
        }
        io_task_sender.send(msg.clone()).await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
