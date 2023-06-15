use std::time::{Duration, Instant};

use futures_util::stream;
use lib::entity::{Msg, Type, HEAD_LEN};
use monoio::{
    fs::OpenOptions,
    io::{AsyncReadRentExt, AsyncWriteRentExt},
    net::{TcpStream, UnixStream},
};
use tracing::{error, info};

pub(crate) async fn test_recv() {
    let stream = UnixStream::connect("/tmp/msglogger.sock").await;
    info!("stream: {:?}", stream);
    if let Err(e) = stream {
        error!("failed to connect to stream: {:?}", e);
        return;
    }
    let mut stream = stream.unwrap();
    let mut msg = Msg::raw(0, 0, 0, vec![0u8; 128].as_slice());
    msg.set_type(Type::RemoteInvoke);
    let mut buffer: Box<[u8; HEAD_LEN]> = Box::new([0; HEAD_LEN]);
    let t = Instant::now();
    let n = 10;
    for _ in 0..n {
        let buf = msg.as_slice()[0..HEAD_LEN].to_owned();
        stream.write_all(buf).await;
        stream.read_u8().await;
    }
    println!("time: {:?}", t.elapsed().as_nanos() / n as u128);
}

#[monoio::main(enable_timer = true)]
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
    // test_recv().await;
    let mut stream = TcpStream::connect("127.0.0.1:50002").await.unwrap();
    for _ in 0..10 {
        monoio::time::sleep(Duration::from_millis(1000)).await;
        stream.write_all(Vec::from("0123456789abcdef")).await;
        let mut buffer = vec![0u8; 16];
        let mut res;
        (res, buffer) = stream.read_exact(buffer).await;
        println!("buffer: {:?}", buffer);
    }
}
