use lib::{joy, Result};
use structopt::StructOpt;
use tracing::{error, info};

use crate::service::handler::{IOTaskReceiver, IOTaskSender};
use crate::{
    config::{CONFIG, CONFIG_FILE_PATH},
    service::handler::IOTaskMsg,
    util::my_id,
};

mod cache;
mod cluster;
mod config;
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

pub(crate) static mut IO_TASK_SENDER: Option<IOTaskSender> = None;

pub(crate) fn get_io_task_sender() -> &'static IOTaskSender {
    unsafe {
        &IO_TASK_SENDER
            .as_ref()
            .expect("io task sender not initialized")
    }
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
        "prim message[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    // todo size optimization
    let (io_task_sender, io_task_receiver) = tokio::sync::mpsc::channel::<IOTaskMsg>(1024);
    unsafe { IO_TASK_SENDER = Some(IOTaskSender(io_task_sender)) };
    tokio::spawn(async move {
        if let Err(e) = cluster::start().await {
            error!("cluster error: {}", e);
        }
    });
    let task_sender = io_task_sender.clone();
    tokio::spawn(async move {
        if let Err(e) = schedule::start(IOTaskSender(task_sender)).await {
            error!("schedule error: {}", e);
        }
    });
    service::start(
        IOTaskSender(io_task_sender),
        IOTaskReceiver(io_task_receiver),
    )
    .await?;
    Ok(())
}
