use std::sync::Arc;
use thread_local::ThreadLocal;
use tracing::Level;
use crate::persistence::persistence_new_seq_num;

mod config;
mod persistence;
mod scheduler;
mod service;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(Level::DEBUG)
        .try_init()
        .unwrap();
    let file_tl = Arc::new(ThreadLocal::new());
    let signal_tl = Arc::new(ThreadLocal::new());
    for i in 0..11 {
        _ = persistence_new_seq_num(&file_tl, &signal_tl, i, i + 1, i);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
