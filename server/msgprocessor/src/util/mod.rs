use std::path::PathBuf;

use lib::{Result, MSGPROCESSOR_ID_BEGINNING};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cache::{get_redis_ops, NODE_ID};

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

pub(crate) async fn load_my_id(my_id_preload: u32) -> Result<()> {
    if my_id_preload != 0 {
        unsafe { MY_ID = my_id_preload };
        return Ok(());
    }
    let path = PathBuf::from("./message/my_id");
    let path = path.as_path();
    let file = tokio::fs::File::open(path).await;
    let my_id;
    if let Ok(file) = file {
        let mut reader = tokio::io::BufReader::new(file);
        let mut s = String::new();
        reader.read_to_string(&mut s).await?;
        my_id = s.parse::<u32>()?;
    } else {
        let mut file = tokio::fs::File::create(path).await?;
        let mut redis_ops = get_redis_ops().await;
        my_id = redis_ops.lua1("local key = KEYS[1] local increment = tonumber(ARGV[1]) local value = redis.call('GET', key) if not value then redis.call('SET', key, increment) return increment else local newValue = redis.call('INCR', key) return newValue end", NODE_ID, MSGPROCESSOR_ID_BEGINNING).await?;
        let s = my_id.to_string();
        file.write_all(s.as_bytes()).await?;
        file.flush().await?;
    }
    unsafe { MY_ID = my_id }
    Ok(())
}

#[inline]
#[allow(unused)]
pub(crate) fn type_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[cfg(test)]
mod tests {
    use lib::Result;

    use crate::cache::get_redis_ops;
    #[tokio::test]
    async fn test() {
        let mut redis_ops = get_redis_ops().await;
        let id: Result<u32> = redis_ops.lua1("local key = KEYS[1] local increment = tonumber(ARGV[1]) local value = redis.call('GET', key) if not value then redis.call('SET', key, increment) return increment else local newValue = redis.call('INCR', key) return newValue end", "TEST_INC", 123).await;
        println!("{:?}", id);
    }
}
