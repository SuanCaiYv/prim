use super::get_cluster_client_map;
use crate::core::{get_connection_map, Result};
use crate::util::my_id;
use anyhow::anyhow;
use common::entity::Type;
use common::net::{OuterReceiver, OuterSender};
use tracing::{debug, error, warn};

pub(super) mod business;
pub(super) mod internal;
pub(super) mod logic;
pub(super) mod message;

const GROUP_ID_THRESHOLD: u64 = 1 << 33;

/// forward and persistence of message was done here.
/// the handlers only handle work about logic.
pub(super) async fn io_tasks(mut receiver: OuterReceiver) -> Result<()> {
    let connection_map = get_connection_map();
    let cluster_client_map = get_cluster_client_map();
    loop {
        let msg = receiver.recv().await;
        if msg.is_none() {
            warn!("global channel closed.");
            return Err(anyhow!("global channel closed."));
        }
        let msg = msg.unwrap();
        match msg.typ() {
            Type::Text
            | Type::Meme
            | Type::File
            | Type::Image
            | Type::Audio
            | Type::Video
            | Type::Echo => {
                let mut should_remove = false;
                let mut existed = false;
                let receiver = msg.receiver();
                {
                    if let Some(sender) = connection_map.0.get(&receiver) {
                        let result = sender.send(msg.clone()).await;
                        if result.is_err() {
                            should_remove = true;
                        }
                        existed = true;
                    }
                }
                {
                    if should_remove {
                        debug!("user: {} maybe offline.", &receiver);
                        connection_map.0.remove(&receiver);
                    }
                }
                let node_id = msg.receiver_node();
                if !existed {
                    if node_id == my_id() {
                        if receiver <= GROUP_ID_THRESHOLD {
                            create_group_task(msg.receiver()).await;
                        } else {
                            debug!("user: {} offline.", &receiver);
                        }
                    } else {
                        let connection = cluster_client_map.get(&node_id);
                        if connection.is_none() {
                            error!("node {} is offline.", node_id);
                            continue;
                        }
                        let connection = connection.unwrap();
                        let res = connection.0.send(msg).await;
                        if res.is_err() {
                            error!("send message to node {} failed.", node_id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

async fn create_group_task(#[allow(unused)] group_id: u64) -> OuterSender {
    todo!()
}
