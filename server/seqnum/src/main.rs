use std::{io::Read, sync::atomic::AtomicU64};

use ahash::AHashMap;
use lib::{joy, Result};
use structopt::StructOpt;
use sysinfo::SystemExt;
use tracing::{error, info, warn};

use crate::{
    config::{CONFIG, CONFIG_FILE_PATH},
    service::{get_seqnum_map, handler::seqnum::SAVE_THRESHOLD},
    util::{from_bytes, load_my_id},
};

mod config;
mod scheduler;
mod service;
mod util;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/seqnum")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./seqnum/config.toml"
    )]
    pub(crate) config: String,
    #[structopt(
        long = "my_id",
        long_help = r"manually set 'my_id' of server node",
        default_value = "1048577"
    )]
    pub(crate) my_id: u32,
}

fn main() {
    let opt: Opt = Opt::from_args();
    let my_id = match std::env::var("MY_ID") {
        Ok(my_id) => my_id.parse::<u32>().unwrap(),
        Err(_) => opt.my_id,
    };
    let config_path = match std::env::var("CONFIG_PATH") {
        Ok(config_path) => config_path,
        Err(_) => opt.config,
    };
    unsafe { CONFIG_FILE_PATH = Box::leak(config_path.into_boxed_str()) }
    load_my_id(my_id).unwrap();
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
        "prim seqnum[{}] running on {}",
        util::my_id(),
        CONFIG.server.service_address
    );
    info!("loading seqnum...");
    load().unwrap();
    info!("loading seqnum done.");
    let sys = sysinfo::System::new_all();
    for _ in 0..sys.cpus().len() - 1 {
        std::thread::spawn(|| {
            #[cfg(target_os = "linux")]
            let _ = monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
                .with_entries(16384)
                .enable_timer()
                .build()
                .unwrap()
                .block_on(service::start());
            #[cfg(target_os = "macos")]
            let _ = monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
                .enable_timer()
                .build()
                .unwrap()
                .block_on(service::start());
        });
    }
    // todo()! save seqnum to file
    // ctrlc::set_handler(move || STOP_SIGNAL.store(true, std::sync::atomic::Ordering::Relaxed))
    //     .expect("Error setting Ctrl-C handler");
    #[cfg(target_os = "linux")]
    let _ = monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
        .with_entries(16384)
        .enable_timer()
        .build()
        .unwrap()
        .block_on(async {
            if let Err(e) = scheduler::start().await {
                error!("scheduler error: {}", e);
            }
            service::start().await
        });
    #[cfg(target_os = "macos")]
    let _ = monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
        .enable_timer()
        .build()
        .unwrap()
        .block_on(async {
            scheduler::start().await.unwrap();
            if let Err(e) = service::start().await {
                error!("scheduler error: {}", e);
            }
        });
}

pub(self) fn load() -> Result<()> {
    let mut map = AHashMap::new();
    let mut buf = vec![0u8; 24];
    // monoio doesn't support async read_dir, but use std is acceptable because
    // this method is only called once at the beginning of the program.
    _ = std::fs::create_dir_all(&CONFIG.server.append_dir);
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
                        warn!("read seqnum file error: {:?}", res);
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
    if CONFIG.server.exactly_mode {
        for (key, seqnum) in map {
            seqnum_map.insert(key, AtomicU64::new(seqnum + 1));
        }
    } else {
        for (key, seqnum) in map {
            seqnum_map.insert(key, AtomicU64::new(seqnum + SAVE_THRESHOLD));
        }
    }
    Ok(())
}
