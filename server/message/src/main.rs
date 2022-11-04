use structopt::StructOpt;
use crate::config::{CONFIG, CONFIG_FILE_PATH};
use common::joy;
use common::Result;
use tracing::info;
use crate::util::MY_ID;

mod cache;
mod config;
mod core;
mod entity;
mod error;
mod util;
mod rpc;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/message")]
pub(crate) struct Opt {
    #[structopt(long, long_help = r"provide you config.toml file by this option", default_value = "./config.toml")]
    pub(crate) config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();
    unsafe {CONFIG_FILE_PATH = Box::leak(opt.config.into_boxed_str())}
    println!("{}", unsafe {CONFIG_FILE_PATH});
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
    util::load_my_id().await?;
    // rpc::gen()?;
    println!("{}", joy::banner());
    // tokio::spawn(async {
    //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    //     let _ = core::mock().await;
    // });
    info!("prim message[{}] running on {}", unsafe { MY_ID }, CONFIG.server.address);
    let _ = core::start().await?;
    Ok(())
}
