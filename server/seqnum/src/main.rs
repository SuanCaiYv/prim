use std::{
    io::{Read, Write},
    time::Instant,
};

use ahash::AHashMap;
use byteorder::{BigEndian, ByteOrder};
use tokio::io::AsyncWriteExt;

use crate::persistance::new_seq_num;

mod config;
mod persistance;
mod scheduler;
mod service;

#[tokio::main]
async fn main() {
    // let mut file = tokio::fs::OpenOptions::new()
    // .create(true)
    //     .append(true)
    //     .custom_flags(0o0040000)
    //     .open("/Users/slma/RustProjects/prim/server/seqnum/test.txt")
    //     .await
    //     .unwrap();
    // let t = Instant::now();
    // let n = 100000;
    // for i in 0..n {
    //     file.write_all(format!("{:022}\n", i).as_bytes())
    //         .await
    //         .unwrap();
    // }
    // println!("avg: {:?}", t.elapsed() / n);
    let t = Instant::now();
    let n = 1000000;
    for i in 0..n {
        new_seq_num(i, i + 2, i).await;
    }
    println!("avg: {:?}", t.elapsed() / n as u32);
    let mut map = AHashMap::new();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open("seqnum.out")
        .unwrap();
    let mut buf = [0u8; 24];
    loop {
        let res = file.read_exact(&mut buf[..]);
        if res.is_err() {
            break;
        }
        let user_id1 = BigEndian::read_u64(&buf[0..8]);
        let user_id2 = BigEndian::read_u64(&buf[8..16]);
        let seq_num = BigEndian::read_u64(&buf[16..24]);
        let key: u128 = ((user_id1 as u128) << 64) + user_id2 as u128;
        map.insert(key, seq_num);
    }
    println!("{}", map.len());
}
