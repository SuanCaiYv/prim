use config::CONFIG;
use lib::{joy, Result};
use tracing::info;

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
    println!("{}", joy::banner());
    info!(
        "prim message[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    load_my_id(0).await?;
    info!("loading seqnum...");
    persistence::load().await?;
    info!("loading seqnum done");
    // scheduler::start().await?;
    // cluster::start().await?;
    service::start().await
}
