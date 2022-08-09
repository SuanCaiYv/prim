use std::time::Duration;
use redis::{aio, FromRedisValue, RedisResult, ToRedisArgs};
use tokio::runtime::Builder;

#[derive(Clone)]
pub struct RedisOps {
    connection: aio::MultiplexedConnection,
}

impl RedisOps {

    pub async fn connection(address: String, port: i32) -> Self {
        let url = format!("redis://{}:{}", address, port);
        let connection = redis::Client::open(url).unwrap().get_multiplexed_async_connection().await.unwrap();
        RedisOps{ connection }
    }

    pub async fn set<T: ToRedisArgs>(&mut self, key: String, value: T) -> RedisResult<()> {
        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_ref<T: ToRedisArgs>(&mut self, key: &'static str, value: T) -> RedisResult<()> {
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_exp<T: ToRedisArgs>(&mut self, key: String, value: T, exp: Duration) -> RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(&key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_exp_ref<T: ToRedisArgs>(&mut self, key: &'static str, value: T, exp: Duration) -> RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn get<T: FromRedisValue>(&mut self, key: String) -> RedisResult<T> {
        redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn get_ref<T: FromRedisValue>(&mut self, key: &'static str) -> RedisResult<T> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn del(&mut self, key: String) -> RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn del_ref(&mut self, key: &'static str) -> RedisResult<()> {
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn push_sort_queue<T: ToRedisArgs>(&mut self, key: String, val: T, score: f64) -> RedisResult<()> {
        redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn peek_sort_queue<T: FromRedisValue>(&mut self, key: String) -> RedisResult<T> {
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

    pub async fn peek_sort_queue_more<T: FromRedisValue>(&mut self, key: String, offset: usize, size: usize) -> RedisResult<Vec<T>> {
        redis::cmd("ZREVRANGEBYSCORE")
            .arg(&key)
            .arg("+inf")
            .arg("-inf")
            .arg("LIMIT")
            .arg(&offset)
            .arg(&size)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn peek_sort_queue_more_and_more<T: FromRedisValue>(&mut self, key: String, offset: usize, size: usize, position: f64, is_backing: bool) -> RedisResult<Vec<T>> {
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

    pub async fn push_set<T: ToRedisArgs>(&mut self, key: String, val: T) -> RedisResult<()> {
        redis::cmd("SADD")
            .arg(&key)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn clear_set(&mut self, key: String) -> RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn atomic_increment(&mut self, key: String) -> RedisResult<u64> {
        redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use redis::RedisResult;
    use crate::Msg;
    use crate::persistence::redis_ops::RedisOps;

    #[tokio::test]
    async fn test() {
        let mut redis_ops = RedisOps::connection("127.0.0.1".to_string(), 6379).await;
        redis_ops.push_sort_queue("key3".to_string(), Msg::default(), 1.0).await.unwrap();
        redis_ops.push_sort_queue("key3".to_string(), Msg::pong(1, 2), 2.0).await.unwrap();
        redis_ops.push_sort_queue("key3".to_string(), Msg::ping(2, 1), 3.0).await.unwrap();
        let v: Vec<Msg> = redis_ops.peek_sort_queue_more("key3".to_string(), 0, 2).await.unwrap();
        println!("{:?}", v)
    }
}