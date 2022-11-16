use lib::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

use crate::config::CONFIG;

/// use singleton instance by it's all clones to share connection between Tasks.
#[allow(unused)]
pub(crate) static REDIS_OPS: OnceCell<RedisOps> = OnceCell::const_new();

#[allow(unused)]
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

pub(crate) static USER_NODE_MAP_KEY: &str = "user_node_map_key_";
pub(crate) static NODE_ID_KEY: &str = "scheduler_node_id";
