use crate::config::CONFIG;
use common::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

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

pub(crate) static TOKEN_KEY: &str = "token_key_";
pub(crate) static USER_NODE_MAP_KEY: &str = "user_node_map_key_";
