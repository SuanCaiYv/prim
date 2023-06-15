//! A example to show how to use UnixStream.

use monoio::{
    io::{AsyncReadRent, AsyncWriteRentExt},
    net::{UnixListener, UnixStream},
};

const ADDRESS: &str = "/tmp/monoio-unix-test.sock";

#[monoio::main(enable_timer = true)]
async fn main() {
    monoio::spawn(async move {
        monoio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut client = UnixStream::connect(ADDRESS).await.unwrap();
        let buf = "hello1";
        let (ret, buf) = client.write_all(buf).await;
        ret.unwrap();
        println!("write {} bytes: {buf:?}", buf.len());
    });

    monoio::spawn(async move {
        monoio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut client = UnixStream::connect(ADDRESS).await.unwrap();
        let buf = "hello2";
        let (ret, buf) = client.write_all(buf).await;
        ret.unwrap();
        println!("write {} bytes: {buf:?}", buf.len());
    });

    std::fs::remove_file(ADDRESS).ok();
    let listener = UnixListener::bind(ADDRESS).unwrap();
    println!("listening on {ADDRESS:?}");
    loop {
        let (mut conn, addr) = listener.accept().await.unwrap();
        println!("accepted a new connection from {addr:?}");
        monoio::spawn(async move {
            let buf = Vec::with_capacity(1024);
            let (ret, buf) = conn.read(buf).await;
            ret.unwrap();
            println!("read {} bytes: {buf:?}", buf.len());
        });
    }

    monoio::time::sleep(std::time::Duration::from_secs(10)).await;
    // clear the socket file
    drop(listener);
    std::fs::remove_file(ADDRESS).ok();
}
