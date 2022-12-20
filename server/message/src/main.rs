use lib::{
    joy,
    net::{InnerSender, OuterReceiver},
    Result,
};
use structopt::StructOpt;
use tracing::{error, info};

use crate::{
    config::{CONFIG, CONFIG_FILE_PATH},
    util::my_id,
};

mod cache;
mod cluster;
mod config;
mod recorder;
mod rpc;
mod schedule;
mod service;
mod util;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/message")]
pub(crate) struct Opt {
    #[structopt(
        long,
        long_help = r"provide you config.toml file by this option",
        default_value = "./message/config.toml"
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
    rpc::gen()?;
    println!("{}", joy::banner());
    info!(
        "prim message[{}] running on {}",
        my_id(),
        CONFIG.server.cluster_address
    );
    // todo size optimization
    let io_task_channel: (InnerSender, OuterReceiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_receiver_side_channel_size * 123);
    // must wait for completed.
    recorder::start().await?;
    tokio::spawn(async move {
        if let Err(e) = cluster::start().await {
            error!("cluster error: {}", e);
        }
    });
    let io_task_sender = io_task_channel.0.clone();
    tokio::spawn(async move {
        if let Err(e) = schedule::start(io_task_sender).await {
            error!("schedule error: {}", e);
        }
    });
    service::start(io_task_channel).await?;
    Ok(())
}
