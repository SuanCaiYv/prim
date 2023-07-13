use std::path::PathBuf;

use lib::{Result, MESSAGE_NODE_ID_BEGINNING};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cache::{get_redis_ops, NODE_ID};

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
#[allow(unused)]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

#[allow(unused)]
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
        let tmp: Result<u64> = redis_ops.get(NODE_ID).await;
        if tmp.is_err() {
            redis_ops.set(NODE_ID, &MESSAGE_NODE_ID_BEGINNING).await?;
        }
        my_id = redis_ops.atomic_increment(NODE_ID).await.unwrap() as u32;
        let s = my_id.to_string();
        file.write_all(s.as_bytes()).await?;
        file.flush().await?;
    }
    unsafe { MY_ID = my_id }
    Ok(())
}

#[inline]
#[allow(unused)]
pub(crate) fn should_connect_to_peer(peer_id: u32, new_peer: bool) -> bool {
    let peer_odd = peer_id & 1 == 1;
    let me_odd = my_id() & 1 == 1;
    if peer_odd && me_odd {
        new_peer
    } else if peer_odd && !me_odd {
        !new_peer
    } else if !peer_odd && me_odd {
        !new_peer
    } else {
        new_peer
    }
}

#[inline]
#[allow(unused)]
pub(crate) fn type_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
