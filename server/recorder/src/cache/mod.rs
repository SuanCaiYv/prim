use std::time::Duration;

use lib::{cache::redis_ops::RedisOps, entity::Msg, util::timestamp, Result};
use tokio::sync::OnceCell;

use crate::{config::CONFIG, rpc::get_rpc_client, util::my_id};

/// use singleton instance by it's all clones to share connection between Tasks.
pub(crate) static REDIS_OPS: OnceCell<RedisOps> = OnceCell::const_new();

pub(super) async fn get_redis_ops() -> RedisOps {
    (REDIS_OPS
        .get_or_init(|| async {
            RedisOps::connect(CONFIG.redis.addresses.clone())
                .await
                .unwrap()
        })
        .await)
        .clone()
}

pub(crate) static NODE_ID: &str = "NODE_ID_RECORDER_";
pub(crate) static MSG_CACHE: &str = "MSG_CACHE_";

pub(crate) async fn make_up_queue_set() -> Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(60 * 60 * 24)).await;
        let mut redis_ops = get_redis_ops().await;
        let mut rpc_client = get_rpc_client().await;
        let (mut node_id_list, _) = rpc_client.call_recorder_list(0).await?;
        // todo optimization
        let keys = redis_ops.keys(&format!("{}*", MSG_CACHE)).await?;
        node_id_list.sort();
        let index = node_id_list.iter().position(|x| *x == my_id()).unwrap_or(0);
        let size = keys.len();
        let len = node_id_list.len();
        let last;
        if size % len == 0 {
            last = size / len;
        } else {
            last = size - ((size / len) + 1) * (len - 1)
        }
        let from;
        let to;
        if index == len - 1 {
            from = size - last;
            to = size;
        } else {
            from = index * (size / len);
            to = from + (size / len);
        }
        let target_timestamp =
            timestamp() - Duration::from_secs(60 * 60 * 24 * 50).as_millis() as u64;
        for i in from..to {
            loop {
                let mut seq_num = 0;
                redis_ops
                    .peek_sort_queue_more::<Msg>(&keys[i], 0, 300, 0.0, f64::MAX, true)
                    .await?
                    .iter()
                    .for_each(|x| {
                        if x.timestamp() < target_timestamp {
                            seq_num = x.seq_num();
                        } else {
                            return;
                        }
                    });
                if seq_num > 0 {
                    redis_ops
                        .remove_sort_queue_old_data(&keys[i], seq_num as f64)
                        .await?;
                } else {
                    break;
                }
            }
        }
    }
}
