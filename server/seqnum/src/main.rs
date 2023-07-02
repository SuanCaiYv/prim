use std::{
    io::Read,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use ahash::AHashMap;
use anyhow::anyhow;
use config::CONFIG;
use lib::{joy, Result};
use monoio::BufResult;
use sysinfo::SystemExt;
use tracing::{error, info};

use crate::service::get_seqnum_map;
use crate::util::{from_bytes, load_my_id, my_id};

mod cache;
mod config;
mod scheduler;
mod service;
mod util;

fn main() {
    let sys = sysinfo::System::new_all();
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init()
        .unwrap();
    println!("{}", joy::banner());
    info!(
        "prim message[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    // load_my_id(0).await?;
    info!("loading seqnum...");
    load().unwrap();
    info!("loading seqnum done");
    for _ in 0..sys.cpus().len() - 1 {
        std::thread::spawn(|| {
            #[cfg(target_os = "linux")]
            monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
                .with_entries(16384)
                .enable_timer()
                .build()
                .unwrap()
                .block_on(service::start());
            #[cfg(target_os = "macos")]
            monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
                .enable_timer()
                .build()
                .unwrap()
                .block_on(service::start());
        });
    }
    #[cfg(target_os = "linux")]
    monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
        .with_entries(16384)
        .enable_timer()
        .build()
        .unwrap()
        .block_on(async {
            // load_my_id(0).await?;
            info!("loading seqnum...");
            load().await?;
            info!("loading seqnum done");
            // scheduler::start().await?;
            service::start().await
        });
    #[cfg(target_os = "macos")]
    monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
        .enable_timer()
        .build()
        .unwrap()
        .block_on(async {
            // scheduler::start().await?;
            service::start().await
        });
}

pub(self) fn load() -> Result<()> {
    let mut map = AHashMap::new();
    let mut buf = vec![0u8; 24];
    // monoio doesn't support async read_dir, but use std is acceptable because
    // this method is only called once at the beginning of the program.
    let mut dir = std::fs::read_dir(&CONFIG.server.append_dir)?;
    while let Some(entry) = dir.next() {
        let file_name = entry?.file_name();
        if let Some(file_name_str) = file_name.to_str() {
            if file_name_str.starts_with("seqnum-") {
                let mut file = std::fs::OpenOptions::new()
                    .read(true)
                    .open(&format!("{}/{}", CONFIG.server.append_dir, file_name_str))?;
                loop {
                    let res = file.read_exact(buf.as_mut_slice());
                    if res.is_err() {
                        error!("read seqnum file error: {:?}", res);
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
            }
        }
    }
    let seqnum_map = get_seqnum_map();
    for (key, seqnum) in map {
        seqnum_map.insert(key, Arc::new(AtomicU64::new(seqnum)));
    }
    Ok(())
}
