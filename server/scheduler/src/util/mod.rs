use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use lib::{Result, SCHEDULER_NODE_ID_BEGINNING};

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
    let path = PathBuf::from("./scheduler/my_id");
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
            redis_ops.set(NODE_ID, &SCHEDULER_NODE_ID_BEGINNING).await?;
        }
        my_id = redis_ops
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
