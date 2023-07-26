use crate::config::CONFIG;
use lib::cache::redis_ops::RedisOps;
use tokio::sync::OnceCell;

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

pub(crate) static USER_TOKEN: &str = "USER_TOKEN_";
pub(crate) static JOIN_GROUP: &str = "JOIN_GROUP_";
pub(crate) static CHECK_CODE : &str = "CHECK_CODE_";
pub(crate) static LAST_ONLINE_TIME: &str = "LAST_ONLINE_TIME_";
pub(crate) static LAST_READ: &str = "LAST_READ_";
pub(crate) static USER_INBOX: &str = "USER_INBOX_";
pub(crate) static MSG_CACHE: &str = "MSG_CACHE_";
pub(crate) static ADD_FRIEND: &str = "ADD_FRIEND_";
