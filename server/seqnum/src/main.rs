use anyhow::Ok;
use config::CONFIG;
use lib::{joy, Result};
use tracing::{info, error};

use crate::util::{load_my_id, my_id};

mod cache;
mod cluster;
mod config;
mod persistence;
mod scheduler;
mod service;
mod util;

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
    // println!("{}", joy::banner());
    // info!(
    //     "prim message[{}] running on {}",
    //     my_id(),
    //     CONFIG.server.service_address
    // );
    // load_my_id(0).await?;
    // tokio::spawn(async move {
    //     service::start().await?;
    //     Result::<()>::Ok(())
    // });
    // scheduler::start().await?;
    // cluster::start().await?;
    for i in 0..1000 {
        tokio::spawn(async move {
            if let Err(e) = persistence::save(((i as u128) << 64 | (i + 1) as u128), i).await {
                error!("save error: {}", e);
            }
        });
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    Ok(())
}
