use tokio::sync::OnceCell;

use crate::cache::redis_ops::RedisOps;

pub(super) mod redis_ops;

/// use singleton instance by it's all clones to share connection between Tasks.
pub(crate) static REDIS_OPS: OnceCell<RedisOps> = OnceCell::const_new();

pub(super) async fn get_redis_ops() -> RedisOps {
    (REDIS_OPS
        .get_or_init(|| async { RedisOps::connect().await.unwrap() })
        .await)
        .clone()
}

pub(crate) static TOKEN_KEY: &str = "token_key_";
