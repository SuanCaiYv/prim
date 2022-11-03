#![feature(map_first_last)]
use crate::config::CONFIG;
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

#[tokio::main]
async fn main() -> Result<()> {
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
    info!("prim server[{}] running...", unsafe { MY_ID });
    println!("{}", joy::banner());
    // tokio::spawn(async {
    //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    //     let _ = core::mock().await;
    // });
    // let _ = core::start().await?;
    Ok(())
}
