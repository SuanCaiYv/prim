mod cache;
mod cluster;
mod config;
mod rpc;
mod service;
mod util;

use lib::{joy, Result};

use structopt::StructOpt;
use tracing::{error, info};
use util::load_my_id;

use crate::config::{CONFIG, CONFIG_FILE_PATH};
use crate::util::my_id;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/scheduler")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./scheduler/config.toml"
    )]
    pub(crate) config: String,
    #[structopt(
        long = "my_id",
        long_help = r"manually set 'my_id' of server node",
        default_value = "0"
    )]
    pub(crate) my_id: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();
    unsafe { CONFIG_FILE_PATH = Box::leak(opt.config.into_boxed_str()) }
    load_my_id(opt.my_id).await?;
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
        "prim scheduler[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    tokio::spawn(async move {
        if let Err(e) = cluster::start().await {
            error!("cluster error: {}", e);
        }
    });
    tokio::spawn(async move {
        if let Err(e) = rpc::start().await {
            error!("rpc error: {}", e);
        }
    });
    service::start().await?;
    Ok(())
}
