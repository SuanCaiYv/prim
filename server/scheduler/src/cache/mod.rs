use lib::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

use crate::config::config;

/// use singleton instance by it's all clones to share connection between Tasks.
#[allow(unused)]
pub(crate) static REDIS_OPS: OnceCell<RedisOps> = OnceCell::const_new();

pub(super) async fn get_redis_ops() -> RedisOps {
    (REDIS_OPS
        .get_or_init(|| async {
            let passwords = if config().redis.passwords.is_empty() {
                None
            } else {
                Some(config().redis.passwords.clone())
            };
            RedisOps::connect(config().redis.addresses.clone(), passwords)
                .await
                .unwrap()
        })
        .await)
        .clone()
}

pub(crate) static USER_NODE_MAP: &str = "USER_NODE_MAP_";
pub(crate) static NODE_ID: &str = "NODE_ID_SCHEDULER_";
