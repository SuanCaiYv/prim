use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::entity::msg::Msg;

mod util;
mod entity;
mod logic;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tokio::spawn(async move {
        logic::connection::bind("127.0.0.1".to_string(), 8190).await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    tokio::spawn(async move {
        let msg = Msg::default();
        let bytes = f(&msg);
        println!("{:?}", bytes.len());
        let stream = TcpStream::connect("127.0.0.1:8190").await;
        stream.unwrap().write(bytes).await
    });
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(())
}

fn f<T: Sized>(p: &T) -> &[u8] {
    unsafe { any_as_u8_slice(p) }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::std::mem::size_of::<T>(),
    )
}
