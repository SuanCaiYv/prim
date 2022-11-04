pub(crate) mod http;
pub(crate) mod rpc;

use crate::cache::{get_redis_ops, USER_NODE_MAP_KEY};
use crate::inner::get_status_map;
use common::Result;
use rand::random;
use tracing::debug;

pub(crate) async fn which_node(user_id: u64) -> Result<u32> {
    let mut redis_ops = get_redis_ops().await;
    let key = format!("{}{}", USER_NODE_MAP_KEY, user_id);
    let value: Result<u32> = redis_ops.get(key.clone()).await;
    match value {
        Ok(value) => Ok(value),
        Err(_) => {
            let status_map = get_status_map().0;
            debug!("status map size: {}", status_map.len());
            loop {
                let index: u32 = random();
                let index = (index as usize) % status_map.len();
                let node_id = status_map.iter().nth(index);
                match node_id {
                    Some(node_id) => {
                        let node_id = node_id.node_id;
                        redis_ops.set(key, node_id).await?;
                        return Ok(node_id);
                    }
                    None => {}
                }
            }
        }
    }
}
