use std::path::PathBuf;

use byteorder::{BigEndian, ByteOrder};
use lib::{Result, SEQNUM_NODE_ID_BEGINNING};

use crate::cache::{get_redis_ops, NODE_ID};

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

pub(crate) async fn load_my_id(my_id_preload: u32) -> Result<()> {
    todo!();
    // if my_id_preload != 0 {
    //     unsafe { MY_ID = my_id_preload };
    //     return Ok(());
    // }
    // let path = PathBuf::from("./seqnum/my_id");
    // let path = path.as_path();
    // let file = tokio::fs::File::open(path).await;
    // let my_id;
    // if let Ok(file) = file {
    //     let mut reader = tokio::io::BufReader::new(file);
    //     let mut s = String::new();
    //     reader.read_to_string(&mut s).await?;
    //     my_id = s.parse::<u32>()?;
    // } else {
    //     let mut file = tokio::fs::File::create(path).await?;
    //     let mut redis_ops = get_redis_ops().await;
    //     let tmp: Result<u64> = redis_ops.get(NODE_ID).await;
    //     if tmp.is_err() {
    //         redis_ops.set(NODE_ID, &SEQNUM_NODE_ID_BEGINNING).await?;
    //     }
    //     my_id = redis_ops.atomic_increment(NODE_ID).await.unwrap() as u32;
    //     let s = my_id.to_string();
    //     file.write_all(s.as_bytes()).await?;
    //     file.flush().await?;
    // }
    // unsafe { MY_ID = my_id }
    Ok(())
}

pub(crate) fn as_bytes(key: u128, seqnum: u64, buf: &mut [u8]) {
    BigEndian::write_u128(&mut buf[0..16], key);
    BigEndian::write_u64(&mut buf[16..24], seqnum);
}

pub(crate) fn from_bytes(buf: &[u8]) -> (u128, u64) {
    (
        BigEndian::read_u128(&buf[0..16]),
        BigEndian::read_u64(&buf[16..24]),
    )
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
