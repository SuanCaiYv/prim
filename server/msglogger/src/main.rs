use tokio::{fs::OpenOptions, time::Instant, io::AsyncWriteExt};



mod logger;
mod recv;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(false)
        .write(true)
        .open("/Volumes/Recorder/msglogger.log")
        .await
        .unwrap();
    let t = Instant::now();
    for i in 0..1_000_000 {
        let val = format!("{:064}", i);
        file.write_all(val.as_bytes()).await.unwrap();
    }
    file.sync_all().await.unwrap();
    println!("write time: {:?}", t.elapsed());
}
