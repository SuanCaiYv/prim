use crate::cache::get_redis_ops;
use crate::core::{get_connection_map, Result};
use crate::util::{which_node, my_id};
use common::entity::Type;
use common::net::OuterReceiver;
use tracing::debug;

use super::get_cluster_client_map;

pub(super) mod logic;
pub(super) mod message;

const GROUP_ID_THRESHOLD: u64 = 1 << 33;

/// forward and persistence of message was done here.
/// the handlers only handle work about logic.
pub(super) async fn io_tasks(mut receiver: OuterReceiver) -> Result<()> {
    let redis_ops = get_redis_ops().await;
    let connection_map = get_connection_map();
    let cluster_client_map = get_cluster_client_map();
    loop {
        let msg = receiver.recv().await;
        if msg.is_none() {
            panic!("global channel closed.");
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
                let receiver = msg.receiver();
                {
                    if let Some(sender) = connection_map.0.get(&receiver) {
                        let result = sender.send(msg).await;
                        if result.is_err() {
                            should_remove = true;
                        }
                    } else {
                        let node_id = which_node(msg.receiver(), &cluster_client_map);
                        if node_id == my_id() {
                            continue;
                        }
                        let connection = cluster_client_map.get(&node_id);
                        if connection.is_none() {
                            debug!("node {} is offline.", node_id);
                            continue;
                        }
                        let connection = connection.unwrap();
                        let _ = connection.0.send(msg).await;
                    }
                }
                {
                    if should_remove {
                        debug!("user: {} maybe offline.", &receiver);
                        connection_map.0.remove(&receiver);
                    }
                }
            }
            _ => {}
        }
    }
}
