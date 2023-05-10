use std::{
    io::{Read, Write},
    os::unix::prelude::OpenOptionsExt,
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use byteorder::{BigEndian, ByteOrder};
use thread_local::ThreadLocal;
use tokio::io::AsyncWriteExt;

use crate::persistance::new_seq_num;

mod config;
mod persistance;
mod scheduler;
mod service;

#[tokio::main]
async fn main() {
    // for i in 0..8 {
    //     std::thread::spawn(move || {
    //         let mut file = std::fs::OpenOptions::new()
    //             .create(true)
    //             .append(true)
    //             .custom_flags(0o0040000)
    //             .open(&format!("seqnum-{}.out", i))
    //             .unwrap();
    //         let t = Instant::now();
    //         let n = 1000000;
    //         for i in 0..n {
    //             file.write(format!("{:022}\n", i).as_bytes());
    //         }
    //         println!("avg: {:?}", t.elapsed() / n);
    //     }).join().unwrap();
    // }
    let tls = Arc::new(ThreadLocal::new());
    let n = 10;
    let m = 24;
    for r in 0..m {
        let tls = tls.clone();
        tokio::spawn(async move {
            let t = Instant::now();
            for i in 0..n {
                new_seq_num(&tls, i, i + 2, i).await;
            }
            println!("avg{}: {:?}", r, t.elapsed() / n as u32);
        });
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
}
