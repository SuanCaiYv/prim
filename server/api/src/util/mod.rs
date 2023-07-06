use std::path::PathBuf;

use lib::Result;
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
    let path = PathBuf::from("./api/my_id");
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
        my_id = get_redis_ops()
            .await
            .atomic_increment(NODE_ID)
            .await
            .unwrap() as u32;
        let s = my_id.to_string();
        file.write_all(s.as_bytes()).await?;
        file.flush().await?;
    }
    unsafe { MY_ID = my_id }
    Ok(())
}

#[allow(unused)]
#[inline]
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

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
