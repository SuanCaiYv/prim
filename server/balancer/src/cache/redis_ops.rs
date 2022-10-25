use redis_cluster_async::{Client, Connection};

use crate::config::CONFIG;
use common::Result;

#[derive(Clone)]
pub(crate) struct RedisOps {
    connection: Connection,
}

impl RedisOps {
    pub(crate) async fn connect() -> Result<RedisOps> {
        let mut addresses = vec![];
        for address in CONFIG.redis.addresses.iter() {
            addresses.push(format!("redis://{}", address));
        }
        let connection = Client::open(addresses)?.get_connection().await?;
        Ok(RedisOps { connection })
    }

    #[allow(unused)]
    pub(crate) async fn set<T: redis::ToRedisArgs>(
        &mut self,
        key: String,
        value: T,
    ) -> redis::RedisResult<()> {
        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn set_ref<T: redis::ToRedisArgs>(
        &mut self,
        key: &'static str,
        value: T,
    ) -> redis::RedisResult<()> {
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn set_exp<T: redis::ToRedisArgs>(
        &mut self,
        key: String,
        value: T,
        exp: std::time::Duration,
    ) -> redis::RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(&key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn set_exp_ref<T: redis::ToRedisArgs>(
        &mut self,
        key: &'static str,
        value: T,
        exp: std::time::Duration,
    ) -> redis::RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn get<T: redis::FromRedisValue>(
        &mut self,
        key: String,
    ) -> redis::RedisResult<T> {
        redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn get_ref<T: redis::FromRedisValue>(
        &mut self,
        key: &'static str,
    ) -> redis::RedisResult<T> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn del(&mut self, key: String) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn del_ref(&mut self, key: &'static str) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn push_sort_queue<T: redis::ToRedisArgs>(
        &mut self,
        key: String,
        val: T,
        score: f64,
    ) -> redis::RedisResult<()> {
        redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn peek_sort_queue<T: redis::FromRedisValue>(
        &mut self,
        key: String,
    ) -> redis::RedisResult<T> {
        redis::cmd("ZREVRANGEBYSCORE")
            .arg(&key)
            .arg("+inf")
            .arg("-inf")
            .arg("LIMIT")
            .arg("0")
            .arg("1")
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn peek_sort_queue_more<T: redis::FromRedisValue>(
        &mut self,
        key: String,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> redis::RedisResult<Vec<T>> {
        if is_backing {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(&key)
                .arg(position)
                .arg("-inf")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(&key)
                .arg("+inf")
                .arg(position)
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        }
    }

    #[allow(unused)]
    pub(crate) async fn peek_sort_queue_more_with_score<T: redis::FromRedisValue>(
        &mut self,
        key: String,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> redis::RedisResult<Vec<(T, f64)>> {
        if is_backing {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(&key)
                .arg(position)
                .arg("-inf")
                .arg("WITHSCORES")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(&key)
                .arg("+inf")
                .arg(position)
                .arg("WITHSCORES")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        }
    }

    #[allow(unused)]
    pub(crate) async fn push_set<T: redis::ToRedisArgs>(
        &mut self,
        key: String,
        val: T,
    ) -> redis::RedisResult<()> {
        redis::cmd("SADD")
            .arg(&key)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn clear_set(&mut self, key: String) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    #[allow(unused)]
    pub(crate) async fn atomic_increment(&mut self, key: String) -> redis::RedisResult<u64> {
        redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::redis_ops::RedisOps;
    use redis::RedisResult;

    #[tokio::test]
    async fn test() {
        let mut redis_ops = RedisOps::connect().await.unwrap();
        redis_ops.set_ref("aaa", "bbb").await.unwrap();
        let v: RedisResult<String> = redis_ops.get_ref("aaa").await;
        println!("{:?}", v);
    }
}
