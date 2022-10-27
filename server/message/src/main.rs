use crate::config::CONFIG;
use common::joy;
use common::Result;
use tracing::info;

mod cache;
mod config;
mod core;
mod entity;
mod error;
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
    info!("prim server running...");
    println!("{}", joy::banner());
    util::load_my_id().await;
    // tokio::spawn(async {
    //     tokio::time::sleep(Duration::from_millis(100)).await;
    //     let _ = core::mock().await;
    // });
    let _ = core::start().await?;
    Ok(())
}
