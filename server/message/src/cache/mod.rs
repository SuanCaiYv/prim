use lib::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

use crate::config::CONFIG;

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

pub(crate) static TOKEN_KEY: &str = "token_key_";
pub(crate) static NODE_ID_KEY: &str = "message_node_id";
pub(crate) static SEQ_NUM_KEY: &str = "seq_num_";
pub(crate) static LAST_ONLINE_TIME_KEY: &str = "last_online_time_";
