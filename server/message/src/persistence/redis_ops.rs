#[derive(Clone)]
pub struct RedisOps {
    connection: redis::aio::MultiplexedConnection,
}

impl RedisOps {

    pub async fn connect(address: String) -> Self {
        let url = format!("redis://{}", address);
        let connection = redis::Client::open(url).unwrap().get_multiplexed_async_connection().await.unwrap();
        RedisOps{ connection }
    }

    pub async fn set<T: redis::ToRedisArgs>(&mut self, key: String, value: T) -> redis::RedisResult<()> {
        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_ref<T: redis::ToRedisArgs>(&mut self, key: &'static str, value: T) -> redis::RedisResult<()> {
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_exp<T: redis::ToRedisArgs>(&mut self, key: String, value: T, exp: std::time::Duration) -> redis::RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(&key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn set_exp_ref<T: redis::ToRedisArgs>(&mut self, key: &'static str, value: T, exp: std::time::Duration) -> redis::RedisResult<()> {
        redis::cmd("PSETEX")
            .arg(key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn get<T: redis::FromRedisValue>(&mut self, key: String) -> redis::RedisResult<T> {
        redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn get_ref<T: redis::FromRedisValue>(&mut self, key: &'static str) -> redis::RedisResult<T> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn del(&mut self, key: String) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn del_ref(&mut self, key: &'static str) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn push_sort_queue<T: redis::ToRedisArgs>(&mut self, key: String, val: T, score: f64) -> redis::RedisResult<()> {
        redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn peek_sort_queue<T: redis::FromRedisValue>(&mut self, key: String) -> redis::RedisResult<T> {
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

    pub async fn peek_sort_queue_more<T: redis::FromRedisValue>(&mut self, key: String, offset: usize, size: usize, is_backing: bool, position: f64) -> redis::RedisResult<Vec<T>> {
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

    pub async fn peek_sort_queue_more_with_score<T: redis::FromRedisValue>(&mut self, key: String, offset: usize, size: usize, is_backing: bool, position: f64) -> redis::RedisResult<Vec<(T, f64)>> {
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

    pub async fn push_set<T: redis::ToRedisArgs>(&mut self, key: String, val: T) -> redis::RedisResult<()> {
        redis::cmd("SADD")
            .arg(&key)
            .arg(&val)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn clear_set(&mut self, key: String) -> redis::RedisResult<()> {
        redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }

    pub async fn atomic_increment(&mut self, key: String) -> redis::RedisResult<u64> {
        redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut self.connection)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc};
    use redis::RedisResult;
    use crate::entity::msg::Msg;
    use crate::persistence::redis_ops;

    #[tokio::test]
    async fn test() {
        println!("{}", u32::MAX);
        let mut ops = redis_ops::RedisOps::connect("127.0.0.1:6379".to_string()).await;
        ops.push_sort_queue("test".to_string(), "aaa", 1.0).await.unwrap();
        ops.push_sort_queue("test".to_string(), "bbb", 2.0).await.unwrap();
        ops.push_sort_queue("test".to_string(), "ccc", 3.0).await.unwrap();
        let res: RedisResult<Vec<(String, f64)>> = ops.peek_sort_queue_more_with_score("test".to_string(), 0, 3, false, 1.0).await;
        println!("{:?}", serde_json::to_string(&res.unwrap()));
        let mut v = vec![1, 2, 3];
        v.reverse();
        println!("{:?}", v);
    }
}