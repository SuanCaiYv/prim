use crate::config::CONFIG;

mod inner;
mod outer;
mod entity;
mod persistence;
mod config;
mod cache;

use common::Result;

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
    inner::start().await?;
    Ok(())
}
