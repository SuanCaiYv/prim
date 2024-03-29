use std::{any::Any, net::SocketAddr};

use crate::{net::GenericParameter, Result};

use anyhow::anyhow;
use redis::{FromRedisValue, RedisResult, ToRedisArgs};
use redis_cluster_async::{Client, Connection};

/// the clone costs for Connection is cheap.
#[derive(Clone)]
pub struct RedisOps {
    connection: Connection,
}

impl RedisOps {
    pub async fn connect(addrs: Vec<SocketAddr>, password_list: Option<Vec<String>>) -> Result<RedisOps> {
        let mut addresses = vec![];
        if password_list.is_some() && password_list.as_ref().unwrap().len() == addrs.len() {
            let passwords = password_list.as_ref().unwrap();
            let mut i = 0;
            for address in addrs.iter() {
                addresses.push(format!("redis://:{}@{}", passwords[i], address));
                i += 1;
            }
        } else {
            for address in addrs.iter() {
                addresses.push(format!("redis://{}", address));
            }
        }
        let connection = Client::open(addresses)?.get_connection().await?;
        Ok(RedisOps { connection })
    }

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

    pub async fn peek_sort_queue_more<T: FromRedisValue>(
        &mut self,
        key: &str,
        offset: usize,
        size: usize,
        from: f64,
        to: f64,
        asc: bool,
    ) -> Result<Vec<T>> {
        let (cmd, from, to) = if asc {
            ("ZRANGEBYSCORE", from, to)
        } else {
            ("ZREVRANGEBYSCORE", to, from)
        };
        let res: RedisResult<Vec<T>> = if from == f64::MIN {
            redis::cmd(cmd)
                .arg(key)
                .arg("-inf")
                .arg(&to)
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else if to == f64::MAX {
            redis::cmd(cmd)
                .arg(key)
                .arg(&from)
                .arg("+inf")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else {
            redis::cmd(cmd)
                .arg(key)
                .arg(&from)
                .arg(&to)
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

    pub async fn peek_sort_queue_more_with_score<T: FromRedisValue>(
        &mut self,
        key: &str,
        offset: usize,
        size: usize,
        from: f64,
        to: f64,
        asc: bool,
    ) -> Result<Vec<(T, f64)>> {
        let (cmd, from, to) = if asc {
            ("ZRANGEBYSCORE", from, to)
        } else {
            ("ZREVRANGEBYSCORE", to, from)
        };
        let res: RedisResult<Vec<(T, f64)>> = if from == f64::MIN {
            redis::cmd(cmd)
                .arg(key)
                .arg("-inf")
                .arg(&to)
                .arg("WITHSCORES")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else if to == f64::MAX {
            redis::cmd(cmd)
                .arg(key)
                .arg(&from)
                .arg("+inf")
                .arg("WITHSCORES")
                .arg("LIMIT")
                .arg(&offset)
                .arg(&size)
                .query_async(&mut self.connection)
                .await
        } else {
            redis::cmd(cmd)
                .arg(key)
                .arg(&from)
                .arg(&to)
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

    pub async fn remove_sort_queue_old_data(&mut self, key: &str, score: f64) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("ZREMRANGEBYSCORE")
            .arg(key)
            .arg("-inf")
            .arg(score)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    pub async fn remove_sort_queue_data(&mut self, key: &str, score: f64) -> Result<()> {
        let res: RedisResult<()> = redis::cmd("ZREMRANGEBYSCORE")
            .arg(key)
            .arg(score)
            .arg(score)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

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

    pub async fn keys(&mut self, pattern: &str) -> Result<Vec<String>> {
        let res: RedisResult<Vec<String>> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut self.connection)
            .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    pub async fn lua1<T: FromRedisValue, Arg1, Arg2>(
        &mut self,
        script: &str,
        argument1: Arg1,
        argument2: Arg2,
    ) -> Result<T>
        where
            Arg1: ToRedisArgs,
            Arg2: ToRedisArgs,
    {
        let res: RedisResult<T> = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(argument1)
            .arg(argument2)
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
    use crate::cache::redis_ops::RedisOps;
    use crate::Result;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test() -> Result<()> {
        let addres = vec!["106.54.221.36:16379", "106.54.221.36:16380", "106.54.221.36:16381"];
        let addresses = addres
            .iter()
            .map(|x| x.parse().expect("parse error"))
            .collect::<Vec<SocketAddr>>();
        let mut redis_ops = RedisOps::connect(addresses, Some(vec!["Redis.123456".to_string(), "Redis.123456".to_string(), "Redis.123456".to_string()])).await?;
        redis_ops.push_sort_queue("test-key", &"aaa", 1.0).await?;
        redis_ops.push_sort_queue("test-key", &"bbb", 2.0).await?;
        redis_ops.push_sort_queue("test-key", &"ccc", 3.0).await?;
        redis_ops.push_sort_queue("test-key", &"ddd", 4.0).await?;
        redis_ops.push_sort_queue("test-key", &"eee", 5.0).await?;
        let res = redis_ops
            .peek_sort_queue_more::<String>("test-key", 0, 3, 1.0, 3.0, false)
            .await?;
        println!("{:?}", res);
        Ok(())
    }
}
