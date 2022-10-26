use std::io::Write;
use std::path::PathBuf;
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::cache::{get_redis_ops, NODE_ID_KEY};

mod time_queue;

pub(crate) async fn my_id() -> u64 {
    let path = PathBuf::from("./my_id");
    let path = path.as_path();
    let file = std::fs::File::open(path);
    let mut my_id: u64 = 0;
    if let Ok(file) = file {
        let mut reader = std::io::BufReader::new(file);
        my_id = reader.read_u64::<byteorder::BigEndian>().unwrap();
    } else {
        let mut file = std::fs::File::create(path).unwrap();
        my_id = get_redis_ops()
            .await
            .atomic_increment(NODE_ID_KEY.to_string())
            .await
            .unwrap();
        file.write_u64::<byteorder::BigEndian>(my_id).unwrap();
        file.flush().unwrap();
    }
    my_id
}

pub(crate) fn which_node(receiver: u64) -> u64 {
    todo!()
}