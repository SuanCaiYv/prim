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
    #[allow(unused)]
    pub async fn connect(addrs: Vec<SocketAddr>) -> Result<RedisOps> {
        let mut addresses = vec![];
        for address in addrs.iter() {
            addresses.push(format!("redis://{}", address));
        }
        let connection = Client::open(addresses)?.get_connection().await?;
        Ok(RedisOps { connection })
    }

    #[allow(unused)]
    pub async fn set<T: ToRedisArgs>(&mut self, key: &str, value: &T) -> Result<()> {
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
        key: &str,
        value: &T,
        exp: std::time::Duration,
    ) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("PSETEX")
            .arg(key)
            .arg(exp.as_millis() as u64)
            .arg(value)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn get<T: FromRedisValue>(&mut self, key: &str) -> Result<T> {
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
    pub async fn del(&mut self, key: &str) -> Result<()> {
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
        key: &str,
        val: &T,
        score: f64,
    ) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("ZADD")
            .arg(key)
            .arg(score)
            .arg(val)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn peek_sort_queue<T: FromRedisValue>(&mut self, key: &str) -> Result<T> {
        let res: RedisResult<T> = redis::cmd("ZREVRANGEBYSCORE")
            .arg(key)
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
        key: &str,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> Result<Vec<T>> {
        let res: RedisResult<Vec<T>> = if is_backing {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(key)
                .arg(position)
                .arg("-inf")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(key)
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
        key: &str,
        offset: usize,
        size: usize,
        is_backing: bool,
        position: f64,
    ) -> Result<Vec<(T, f64)>> {
        let res: RedisResult<Vec<(T, f64)>> = if is_backing {
            redis::cmd("ZREVRANGEBYSCORE")
                .arg(key)
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
                .arg(key)
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
    pub async fn push_set<T: ToRedisArgs>(&mut self, key: &str, val: &T) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("SADD")
            .arg(key)
            .arg(val)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    #[allow(unused)]
    pub async fn clear_set(&mut self, key: &str) -> RedisResult<()> {
        let res: RedisResult<()> = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[allow(unused)]
    pub async fn atomic_increment(&mut self, key: &str) -> Result<u64> {
        let res: RedisResult<u64> = redis::cmd("INCR")
            .arg(key)
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
    use std::net::SocketAddr;
    use crate::cache::redis_ops::RedisOps;
    use crate::Result;

    #[tokio::test]
    async fn test() -> Result<()> {
        let addres = vec!["127.0.0.1:16379", "127.0.0.1:16380", "127.0.0.1:16381"];
        let addresses = addres.iter().map(|x| x.parse().expect("parse error")).collect::<Vec<SocketAddr>>();
        let mut redis_ops = RedisOps::connect(addresses).await?;
        // redis.set_ref("test", 1 as u64).await?;
        // let v = redis.atomic_increment("test".to_string()).await?;
        let v: Result<()> = redis_ops.set("test", &b"aaa").await;
        println!("{:?}", v);
        Ok(())
    }
}
