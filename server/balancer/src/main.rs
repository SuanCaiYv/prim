use tracing::info;
use crate::config::CONFIG;
use common::Result;

mod cache;
mod config;
mod entity;
mod inner;
mod outer;
mod persistence;

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
        let _ = outer::rpc::start().await;
    });
    info!("prim balancer is running...");
    inner::start().await?;
    Ok(())
}
