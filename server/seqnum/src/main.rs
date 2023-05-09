use std::{time::Instant, io::Write};

use tokio::{io::AsyncWriteExt};

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
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/Users/slma/RustProjects/prim/server/seqnum/test.txt")
        .unwrap();
    let t = Instant::now();
    let n = 1000000;
    for i in 0..n {
        file.write_all(format!("{:06}\n", i).as_bytes())
            .unwrap();
    }
    println!("avg: {:?}", t.elapsed() / n);
}
