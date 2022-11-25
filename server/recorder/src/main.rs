mod cache;
mod config;
mod entity;
mod schedule;
mod service;
mod util;
mod sql;

use lib::{joy, Result};

use structopt::StructOpt;
use tracing::{error, info};

use crate::{
    config::{CONFIG, CONFIG_FILE_PATH},
    util::MY_ID,
};

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/recorder")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./config.toml"
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
    util::load_my_id(opt.my_id).await?;
    // rpc::gen()?;
    println!("{}", joy::banner());
    info!(
        "prim recorder[{}] running on {}",
        unsafe { MY_ID },
        CONFIG.server.service_address
    );
    tokio::spawn(async move {
        if let Err(e) = schedule::start().await {
            error!("schedule error: {}", e);
        }
    });
    service::start().await?;
    Ok(())
}
