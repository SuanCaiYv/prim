use std::{fs, os::unix::prelude::OpenOptionsExt};

use chrono::{Duration, Local, NaiveTime};
use lib::{
    entity::{Head, Msg, HEAD_LEN},
    Result,
};
use local_sync::mpsc;
use monoio::{
    fs::File,
    io::{AsyncReadRentExt, AsyncWriteRentExt},
    net::{UnixListener, UnixStream},
};
use tracing::{error, info};

use crate::logger;

pub(crate) async fn start(id: usize) -> Result<()> {
    let (tx, rx) = mpsc::bounded::channel(1);
    monoio::spawn(async move {
        loop {
            let prefix = Local::now()
                .date_naive()
                .format("%Y-%m-%d")
                .to_string();
            let path = format!("./msglog/{}-{}.log", prefix, id);
            let file = monoio::fs::OpenOptions::new()
                .append(true)
                .custom_flags(0x4000)
                .create(true)
                .open(path)
                .await.unwrap();
            _ = tx.send(file).await;
            let now = Local::now();
            let one_day = Duration::days(1);
            let target_date = now.date_naive() + one_day;
            let target_time = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
            let target_datetime = target_date.and_time(target_time);
            let duration = target_datetime.signed_duration_since(now.naive_local());
            let milliseconds = duration.num_milliseconds();
            monoio::time::sleep(monoio::time::Duration::from_millis(milliseconds as u64)).await;
            _ = logger::clear_log(id);
        }
    });
    let socket_path = format!("/tmp/msglogger-{}.sock", id);
    _ = fs::remove_file(&socket_path);
    let listener = UnixListener::bind(socket_path)?;
    let (stream, addr) = listener.accept().await.unwrap();
    info!("accepted connection from {:?}", addr);
    handle_connection(stream, rx).await
}

pub(self) async fn handle_connection(
    mut stream: UnixStream,
    mut rx: mpsc::bounded::Rx<File>,
) -> Result<()> {
    let mut head_buf = vec![0; HEAD_LEN];
    let mut res;
    let mut file: Option<File> = Some(rx.recv().await.unwrap());
    loop {
        (res, head_buf) = stream.read_exact(head_buf).await;
        if res.is_err() {
            error!("read head error: {:?}", res);
            break;
        }
        let mut head = Head::from(head_buf.as_slice());
        let mut msg = Msg::pre_alloc(&mut head);
        let mut body = vec![0; msg.payload_length() + msg.extension_length()];
        (res, body) = stream.read_exact(body).await;
        if res.is_err() {
            error!("read body error: {:?}", res);
            break;
        }
        msg.0[HEAD_LEN..].copy_from_slice(&body);
        if rx.try_recv().is_ok() {
            file = Some(rx.recv().await.unwrap());
        }
        logger::logger(msg, file.as_mut().unwrap()).await?;
        _ = stream.write_all("ok".as_bytes().to_vec()).await;
    }
    Ok(())
}
