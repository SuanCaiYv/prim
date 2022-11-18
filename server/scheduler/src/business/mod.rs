use lib::Result;
use rand::random;
use tracing::debug;
use anyhow::anyhow;

use crate::{cache::{get_redis_ops, USER_NODE_MAP_KEY}, service::get_client_sender_timeout_receiver_map};

pub(crate) async fn which_node(user_id: u64) -> Result<u32> {
    let mut redis_ops = get_redis_ops().await;
    let key = format!("{}{}", USER_NODE_MAP_KEY, user_id);
    let value: Result<u32> = redis_ops.get(key.clone()).await;
    match value {
        Ok(value) => Ok(value),
        Err(_) => {
            let client_map = get_client_sender_timeout_receiver_map().0;
            debug!("status map size: {}", client_map.len());
            loop {
                let index: u32 = random();
                let index = (index as usize) % client_map.len();
                match client_map.iter().nth(index) {
                    Some(entry) => {
                        let node_id = entry.key();
                        debug!("chosen node: {}", node_id);
                        redis_ops.set(key, node_id).await?;
                        return Ok(*node_id);
                    },
                    None => {
                        return Err(anyhow!("no node available"));
                    },
                }
            }
        }
    }
}