use std::{os::unix::fs::OpenOptionsExt, fs};

use byteorder::{BigEndian, ByteOrder};
use chrono::{Duration, Local, NaiveTime};
use lib::{
    entity::{Head, Msg, HEAD_LEN},
    Result,
};
use local_sync::mpsc;
use monoio::{
    fs::File,
    io::{AsyncReadRentExt, AsyncWriteRentExt, Splitable},
    net::UnixListener,
};
use tracing::{error, info};

use crate::logger;

pub(crate) async fn start(id: usize) -> Result<()> {
    let (tx, rx) = mpsc::bounded::channel(1);
    monoio::spawn(async move {
        loop {
            let prefix = Local::now().date_naive().format("%Y-%m-%d").to_string();
            let path = format!("./msglog/{}-{}.log", prefix, id);
            // todo! bug fix.
            // when you are running on docker, this one will panic, and std::fs will be used.
            // but if you are running on raw host, it works well with std::fs panic.
            let file = monoio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .custom_flags(0x0400)
                .open(&path)
                .await
                .unwrap();
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
    let (mut reader, mut writer) = stream.into_split();
    let (send_sender, send_receiver) = mpsc::bounded::channel(16384);
    let (recv_sender, mut recv_receiver) = mpsc::bounded::channel(16384);
    monoio::spawn(async move {
        let mut id_buf = vec![0u8; 8];
        let mut head_buf = vec![0; HEAD_LEN];
        let mut res;
        loop {
            (res, id_buf) = reader.read_exact(id_buf).await;
            if res.is_err() {
                error!("read id error: {:?}", res);
                break;
            }
            let id = BigEndian::read_u64(&id_buf);
            (res, head_buf) = reader.read_exact(head_buf).await;
            if res.is_err() {
                error!("read head error: {:?}", res);
                break;
            }
            let mut head = Head::from(head_buf.as_slice());
            let mut msg = Msg::pre_alloc(&mut head);
            let mut body = vec![0; msg.payload_length() + msg.extension_length()];
            (res, body) = reader.read_exact(body).await;
            if res.is_err() {
                error!("read body error: {:?}", res);
                break;
            }
            msg.0[HEAD_LEN..].copy_from_slice(&body);
            _ = send_sender.send((id, msg)).await;
        }
    });
    monoio::spawn(async move {
        let mut id_buf = vec![0u8; 8];
        let mut res;
        loop {
            let id = match recv_receiver.recv().await {
                Some(msg) => msg,
                None => break,
            };
            BigEndian::write_u64(&mut id_buf, id);
            (res, id_buf) = writer.write_all(id_buf).await;
            if res.is_err() {
                error!("write id error: {:?}", res);
                break;
            }
        }
    });
    handle_connection(send_receiver, recv_sender, rx).await
}

pub(self) async fn handle_connection(
    mut receiver: mpsc::bounded::Rx<(u64, Msg)>,
    sender: mpsc::bounded::Tx<u64>,
    mut rx: mpsc::bounded::Rx<File>,
) -> Result<()> {
    let mut file: Option<File> = Some(rx.recv().await.unwrap());
    loop {
        let (id, msg) = match receiver.recv().await {
            Some(msg) => msg,
            None => break,
        };
        if rx.try_recv().is_ok() {
            file = Some(rx.recv().await.unwrap());
        }
        if let Err(e) = logger::logger(msg, file.as_mut().unwrap()).await {
            error!("logger error: {:?}", e);
            break;
        };
        _ = sender.send(id).await;
    }
    Ok(())
}
