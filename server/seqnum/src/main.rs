use anyhow::Ok;
use config::CONFIG;

use lib::Result;

mod config;
mod persistence;
mod scheduler;
mod service;
mod util;
mod cache;
mod cluster;

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
    tokio::spawn(async move {
        service::start().await?;
        Result::<()>::Ok(())
    });
    scheduler::start().await?;
    cluster::start().await?;
    Ok(())
}
