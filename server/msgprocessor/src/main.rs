use lib::{Result, joy};
use tracing::info;

use crate::{config::CONFIG, util::my_id};

pub(crate) mod cache;
pub(crate) mod config;
pub(crate) mod model;
pub(crate) mod mq;
pub(crate) mod scheduler;
pub(crate) mod util;

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
    util::load_my_id(0).await?;
    // rpc::gen()?;
    println!("{}", joy::banner());
    info!(
        "prim msgprocessor[{}] running on",
        my_id()
    );
    Ok(())
}
