use config::CONFIG;
use lib::Result;

mod cache;
mod config;
mod echo;
mod group;
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
    tokio::spawn(async move {
        _ = echo::start().await;
    });
    group::start().await
}
