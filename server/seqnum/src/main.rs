use config::CONFIG;

mod config;
mod persistence;
mod scheduler;
mod service;
mod util;
mod cache;

#[tokio::main]
async fn main() {
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
    // let file_tl = Arc::new(ThreadLocal::new());
    // for i in 0..5000 {
    //     _ = persistence_sequence_number_threshold(&file_tl, i % 3, i % 3 + 1, i).await;
    // }
    // tokio::time::sleep(Duration::from_secs(5)).await;
}
