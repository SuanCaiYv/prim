use crate::net::server::GenericParameter;
use crate::Result;
use anyhow::anyhow;
use redis::{FromRedisValue, RedisResult, ToRedisArgs};
use redis_cluster_async::{Client, Connection};
use std::any::Any;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct RedisOps {
    connection: Connection,
}

impl RedisOps {
    pub async fn connect(addrs: Vec<SocketAddr>) -> Result<RedisOps> {
        let mut addresses = vec![];
        for address in addrs.iter() {
            addresses.push(format!("redis://{}", address));
        }
        let connection = Client::open(addresses)?.get_connection().await?;
        Ok(RedisOps { connection })
    }

    #[allow(unused)]
    pub async fn set<T: ToRedisArgs>(&mut self, key: String, value: T) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn set_ref<T: ToRedisArgs>(&mut self, key: &'static str, value: T) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn set_exp<T: ToRedisArgs>(
        &mut self,
        key: String,
        value: T,
        exp: std::time::Duration,
    ) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("PSETEX")
            .arg(&key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn set_exp_ref<T: ToRedisArgs>(
        &mut self,
        key: &'static str,
        value: T,
        exp: std::time::Duration,
    ) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("PSETEX")
            .arg(key)
            .arg(exp.as_millis() as u64)
            .arg(&value)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn get<T: FromRedisValue>(&mut self, key: String) -> Result<T> {
        let res: RedisResult<T> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn get_ref<T: FromRedisValue>(&mut self, key: &'static str) -> Result<T> {
        let res: RedisResult<T> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn del(&mut self, key: String) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn del_ref(&mut self, key: &'static str) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn push_sort_queue<T: ToRedisArgs>(
        &mut self,
        key: String,
        val: T,
        score: f64,
    ) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("ZADD")
            .arg(&key)
            .arg(score)
            .arg(&val)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn peek_sort_queue<T: FromRedisValue>(&mut self, key: String) -> Result<T> {
        let res: RedisResult<T> = redis::cmd("ZREVRANGEBYSCORE")
            .arg(&key)
            .arg("+inf")
            .arg("-inf")
            .arg("LIMIT")
            .arg("0")
            .arg("1")
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn peek_sort_queue_more<T: FromRedisValue>(
        &mut self,
        key: String,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> Result<Vec<T>> {
        let res: RedisResult<Vec<T>> = if is_backing {
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
        };
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn peek_sort_queue_more_with_score<T: FromRedisValue>(
        &mut self,
        key: String,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> Result<Vec<(T, f64)>> {
        let res: RedisResult<Vec<(T, f64)>> = if is_backing {
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
        };
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn push_set<T: ToRedisArgs>(&mut self, key: String, val: T) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("SADD")
            .arg(&key)
            .arg(&val)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn clear_set(&mut self, key: String) -> RedisResult<()> {
        let res: RedisResult<()> = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[allow(unused)]
    pub async fn atomic_increment(&mut self, key: String) -> Result<u64> {
        let res: RedisResult<u64> = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }
}

impl GenericParameter for RedisOps {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {}
}
