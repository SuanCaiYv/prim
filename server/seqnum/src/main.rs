use std::{time::Duration, sync::{Arc, atomic::AtomicU64}};

use config::CONFIG;
use lib::{joy, Result};
use tracing::info;
use ahash::AHashMap;
use anyhow::anyhow;
use monoio::BufResult;

use crate::service::get_seqnum_map;
use crate::util::{from_bytes, load_my_id, my_id};

mod cache;
mod cluster;
mod config;
mod persistence;
mod scheduler;
mod service;
mod util;

#[monoio::main(enable_timer = true, threads = 2)]
async fn main() {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init();
    // println!("{}", joy::banner());
    // info!(
    //     "prim message[{}] running on {}",
    //     my_id(),
    //     CONFIG.server.service_address
    // );
    // load_my_id(0).await?;
    // info!("loading seqnum...");
    // persistence::load().await?;
    // info!("loading seqnum done");
    // scheduler::start().await?;
    // cluster::start().await?;
    // service::start().await
}

pub(self) async fn load() -> Result<()> {
    let mut map = AHashMap::new();
    let mut buf = vec![0u8; 24];
    let mut res;
    // monoio doesn't support async read_dir, but use std is acceptable because
    // this method is only called once at the beginning of the program.
    let mut dir = std::fs::read_dir(&CONFIG.server.append_dir)?;
    while let Some(entry) = dir.next() {
        let file_name = entry?.file_name();
        if let Some(file_name_str) = file_name.to_str() {
            if file_name_str.starts_with("seqnum-") {
                let mut file = monoio::fs::OpenOptions::new()
                    .read(true)
                    .open(&file_name)
                    .await?;
                loop {
                    (res, buf) = file.read_exact_at(buf, 0).await;
                    if res.is_err() {
                        break;
                    }
                    let (key, seq_num) = from_bytes(&buf[..]);
                    map.entry(key)
                        .and_modify(|seqnum| {
                            if *seqnum < seq_num {
                                *seqnum = seq_num;
                            }
                        })
                        .or_insert(seq_num);
                }
                std::fs::rename(&file_name, file_name_str.replace("seqnum-", "seqnum-bkg"))?;
            }
        }
    }
    let seqnum_map = get_seqnum_map();
    for (key, seqnum) in map {
        seqnum_map.insert(key, Arc::new(AtomicU64::new(seqnum)));
    }
    Ok(())
}
