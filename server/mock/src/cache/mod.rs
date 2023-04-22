use lib::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

use crate::config::CONFIG;

/// use singleton instance by it's all clones to share connection between Tasks.
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

#[allow(unused)]
pub(crate) static USER_TOKEN: &str = "USER_TOKEN_";
#[allow(unused)]
pub(crate) static NODE_ID: &str = "NODE_ID_MESSAGE_";
#[allow(unused)]
pub(crate) static SEQ_NUM: &str = "SEQ_NUM_";
#[allow(unused)]
pub(crate) static MSG_CACHE: &str = "MSG_CACHE_";
#[allow(unused)]
pub(crate) static LAST_ONLINE_TIME: &str = "LAST_ONLINE_TIME_";
#[allow(unused)]
pub(crate) static USER_INBOX: &str = "USER_INBOX_";
