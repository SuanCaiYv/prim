use std::time::{Instant, Duration};

use lib::entity::{Head, Msg, Type, HEAD_LEN};

use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use tracing::error;

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
    if let Err(e) = fs::remove_file("/tmp/msglogger.sock").await {
        error!("failed to remove file: {:?}", e);
    }
    let listener = UnixListener::bind("/tmp/msglogger.sock");
    if let Err(e) = listener {
        error!("failed to bind listener: {:?}", e);
        return;
    }
    let listener = listener.unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let stream = UnixStream::connect("/tmp/msglogger.sock").await;
        if let Err(e) = stream {
            error!("failed to connect to stream: {:?}", e);
            return;
        }
        let mut stream = stream.unwrap();
        let mut msg = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
        msg.set_type(Type::RemoteInvoke);
        let t = Instant::now();
        for _ in 0..1000 {
            stream.write_all(msg.as_slice()).await.unwrap();
        }
        println!("time: {:?}", t.elapsed());
    });
    let stream = listener.accept().await.unwrap();
    let (mut stream, _) = stream;
    let mut buffer: [u8; HEAD_LEN] = [0; HEAD_LEN];
    for _ in 0..1000 {
        match stream.read_exact(&mut buffer).await {
            Ok(_) => {}
            Err(_) => {
                error!("failed to read head from stream.");
                break;
            }
        };
        let mut head = Head::from(&buffer[..]);
        let mut msg = Msg::pre_alloc(&mut head);
        match stream.read_exact(msg.as_mut_body()).await {
            Ok(_) => {}
            Err(_) => {
                error!("failed to read body from stream.");
                break;
            }
        }
    }
    // tokio::spawn(start_recv());
    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // test_recv().await;
    // tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
