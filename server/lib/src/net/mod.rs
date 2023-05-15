pub mod client;
pub mod server;

use ahash::AHashSet;
use anyhow::anyhow;
use async_recursion::async_recursion;
use byteorder::{BigEndian, ByteOrder};

use dashmap::DashMap;
use futures::{pin_mut, select, Future, FutureExt};
use quinn::{ReadExactError, RecvStream, SendStream};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    time::{Instant, Sleep},
};
use tokio_rustls::{client as tls_client, server as tls_server};
use tracing::{debug, error, info};

use crate::{
    entity::{
        msg::MSG_DELIMITER, Head, Msg, ReqwestMsg, TinyMsg, Type, EXTENSION_THRESHOLD, HEAD_LEN,
        PAYLOAD_THRESHOLD,
    },
    Result,
};

/// the direction is relative to the stream task.
///
/// why tokio? cause this direction's model is multi-sender and single-receiver
///
/// why async-channel? cause this direction's model is single-sender multi-receiver
pub type MsgMpmcReceiver = async_channel::Receiver<Arc<Msg>>;
pub type MsgMpmcSender = async_channel::Sender<Arc<Msg>>;
pub type MsgMpscSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type MsgMpscReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
pub(self) const TIMEOUT_WHEEL_SIZE: u64 = 4096;

#[inline(always)]
pub(self) fn pre_check(msg: &[u8]) -> usize {
    if msg.len() < 4 {
        return msg.len();
    }
    // by test, directly iter is fast enough to work.
    let mut i = 0;
    while i < msg.len() - 3 {
        if msg[i] == MSG_DELIMITER[0] {
            if msg[i + 1] == MSG_DELIMITER[1] {
                if msg[i + 2] == MSG_DELIMITER[2] {
                    if msg[i + 3] == MSG_DELIMITER[3] {
                        return i;
                    } else {
                        i += 4;
                    }
                } else {
                    i += 3;
                }
            } else {
                i += 2;
            }
        } else {
            i += 1;
        }
    }
    msg.len()
}

#[derive(Clone)]
pub enum MsgSender {
    Client(MsgMpmcSender),
    Server(MsgMpscSender),
}

impl MsgSender {
    pub async fn send(&self, msg: Arc<Msg>) -> Result<()> {
        match self {
            MsgSender::Client(sender) => {
                sender.send(msg).await?;
            }
            MsgSender::Server(sender) => {
                sender.send(msg).await?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct TimerSetter {
    sender: tokio::sync::mpsc::Sender<Instant>,
}

impl TimerSetter {
    fn new(sender: tokio::sync::mpsc::Sender<Instant>) -> Self {
        Self { sender }
    }

    async fn set(&self, timeout: Instant) {
        _ = self.sender.send(timeout).await;
    }
}

pub(self) struct SharedTimer {
    timer: Pin<Box<Sleep>>,
    task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    set_sender: tokio::sync::mpsc::Sender<Instant>,
    set_receiver: tokio::sync::mpsc::Receiver<Instant>,
}

impl SharedTimer {
    fn new(timeout: Duration, callback: impl Future<Output = ()> + Send + 'static) -> Self {
        let timer = tokio::time::sleep(timeout);
        let (set_sender, set_receiver) = tokio::sync::mpsc::channel(1);
        Self {
            timer: Box::pin(timer),
            task: Box::pin(callback),
            set_sender,
            set_receiver,
        }
    }

    fn setter(&self) -> TimerSetter {
        TimerSetter::new(self.set_sender.clone())
    }
}

impl Unpin for SharedTimer {}

impl Future for SharedTimer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        match self.set_receiver.poll_recv(cx) {
            Poll::Pending => match self.timer.as_mut().poll(cx) {
                Poll::Ready(_) => match self.task.as_mut().poll(cx) {
                    Poll::Ready(_) => Poll::Ready(()),
                    Poll::Pending => Poll::Pending,
                },
                Poll::Pending => Poll::Pending,
            },
            Poll::Ready(Some(timeout)) => {
                self.timer.as_mut().reset(timeout);
                match self.timer.as_mut().poll(cx) {
                    Poll::Ready(_) => match self.task.as_mut().poll(cx) {
                        Poll::Ready(_) => Poll::Ready(()),
                        Poll::Pending => Poll::Pending,
                    },
                    Poll::Pending => Poll::Pending,
                }
            }
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}

/// read bytes from stream, if external_source is not None, read from external_source first,
/// and return the rest of external_source if remained.
#[inline(always)]
pub(self) async fn read_buffer<'a>(
    recv_stream: &mut RecvStream,
    external_source: Option<&'a [u8]>,
    buffer: &mut [u8],
) -> Result<Option<&'a [u8]>> {
    match external_source {
        Some(external_source) => {
            if external_source.len() < buffer.len() {
                buffer[0..external_source.len()].copy_from_slice(external_source);
                match recv_stream
                    .read_exact(&mut buffer[external_source.len()..])
                    .await
                {
                    Ok(_) => Ok(None),
                    Err(e) => match e {
                        ReadExactError::FinishedEarly => {
                            info!("stream finished.");
                            Err(anyhow!(crate::error::CrashError::ShouldCrash(
                                "stream finished.".to_string()
                            )))
                        }
                        ReadExactError::ReadError(e) => {
                            debug!("read stream error: {:?}", e);
                            Err(anyhow!(crate::error::CrashError::ShouldCrash(
                                "read stream error.".to_string()
                            )))
                        }
                    },
                }
            } else {
                buffer.copy_from_slice(&external_source[0..buffer.len()]);
                Ok(Some(&external_source[buffer.len()..]))
            }
        }
        None => match recv_stream.read_exact(buffer).await {
            Ok(_) => Ok(None),
            Err(e) => match e {
                ReadExactError::FinishedEarly => {
                    info!("stream finished.");
                    Err(anyhow!(crate::error::CrashError::ShouldCrash(
                        "stream finished.".to_string()
                    )))
                }
                ReadExactError::ReadError(e) => {
                    debug!("read stream error: {:?}", e);
                    Err(anyhow!(crate::error::CrashError::ShouldCrash(
                        "read stream error.".to_string()
                    )))
                }
            },
        },
    }
}

pub(self) struct MsgIOUtil;

impl MsgIOUtil {
    /// the only error returned should cause the stream crashed.
    ///
    /// the purpose using [`std::sync::Arc`] is to reduce unnecessary memory copy.
    #[async_recursion]
    pub(self) async fn recv_msg<'a: 'async_recursion>(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut RecvStream,
        mut external_source: Option<&'a [u8]>,
    ) -> Result<Arc<Msg>> {
        #[cfg(feature = "pre-check")]
        {
            let mut from = 0;
            let mut delimiter_buf = [0u8; 4];
            let mut loss = 0;
            loop {
                match read_buffer(recv_stream, external_source, &mut delimiter_buf[from..]).await {
                    Ok(external_source0) => {
                        external_source = external_source0;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                if delimiter_buf[0] != MSG_DELIMITER[0] {
                    loss += 1;
                    debug!("invalid message detected[1].");
                    delimiter_buf[0] = delimiter_buf[1];
                    delimiter_buf[1] = delimiter_buf[2];
                    delimiter_buf[2] = delimiter_buf[3];
                    from = 3;
                    continue;
                } else if delimiter_buf[1] != MSG_DELIMITER[1] {
                    loss += 2;
                    debug!("invalid message detected[2].");
                    delimiter_buf[0] = delimiter_buf[2];
                    delimiter_buf[1] = delimiter_buf[3];
                    from = 2;
                    continue;
                } else if delimiter_buf[2] != MSG_DELIMITER[2] {
                    loss += 3;
                    debug!("invalid message detected[3].");
                    delimiter_buf[0] = delimiter_buf[3];
                    from = 1;
                    continue;
                } else if delimiter_buf[3] != MSG_DELIMITER[3] {
                    loss += 4;
                    debug!("invalid message detected[4].");
                    from = 0;
                } else {
                    break;
                }
            }
            if loss != 0 {
                error!(
                    "{} message loss {} bytes detected.",
                    recv_stream.id().0,
                    loss
                );
            }
        }
        match read_buffer(recv_stream, external_source, &mut buffer[..]).await {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(feature = "pre-check")]
        {
            let index = pre_check(&buffer[..]);
            if index != buffer.len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&buffer[index..]);
                let res = MsgIOUtil::recv_msg(buffer, recv_stream, Some(&external)).await;
                return res;
            }
        }
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut head = Head::from(&buffer[..]);
        let mut msg = Msg::pre_alloc(&mut head);
        match read_buffer(
            recv_stream,
            external_source,
            &mut msg.as_mut_slice()[HEAD_LEN..],
        )
        .await
        {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(feature = "pre-check")]
        {
            let index = pre_check(msg.as_slice());
            if index != msg.as_slice().len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&msg.as_slice()[index..]);
                let res = MsgIOUtil::recv_msg(buffer, recv_stream, Some(&external)).await;
                return res;
            }
        }
        Ok(Arc::new(msg))
    }

    #[inline(always)]
    pub(self) async fn recv_msg_server(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut ReadHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<Arc<Msg>> {
        match recv_stream.read_exact(&mut buffer[..]).await {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let mut head = Head::from(&buffer[..]);
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(&mut head);
        match recv_stream
            .read_exact(&mut (msg.as_mut_slice()[HEAD_LEN..]))
            .await
        {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        debug!("read msg: {}", msg);
        Ok(Arc::new(msg))
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn recv_msg_client(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut ReadHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<Arc<Msg>> {
        match recv_stream.read_exact(&mut buffer[..]).await {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let mut head = Head::from(&buffer[..]);
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(&mut head);
        match recv_stream
            .read_exact(&mut (msg.as_mut_slice()[HEAD_LEN..]))
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        debug!("read msg: {}", msg);
        Ok(Arc::new(msg))
    }

    /// the only error returned should cause the stream crashed.
    /// and this method will automatically finish the stream.
    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msg(msg: Arc<Msg>, send_stream: &mut SendStream) -> Result<()> {
        #[cfg(feature = "pre-check")]
        if pre_check(msg.as_slice()) != msg.as_slice().len() {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "invalid message.".to_string()
            )));
        }
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.finish().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msg_server(
        msg: Arc<Msg>,
        send_stream: &mut WriteHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msg_client(
        msg: Arc<Msg>,
        send_stream: &mut WriteHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }
}

pub struct MsgIOWrapper {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
}

impl MsgIOWrapper {
    pub(self) fn new(mut send_stream: SendStream, mut recv_stream: RecvStream) -> Self {
        // actually channel buffer size set to 1 is more intuitive.
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            tokio::sync::mpsc::channel(16384);
        let (recv_sender, recv_receiver): (MsgMpscSender, MsgMpscReceiver) =
            tokio::sync::mpsc::channel(16284);
        tokio::spawn(async move {
            let task1 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) = MsgIOUtil::send_msg(msg, &mut send_stream).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                let mut buffer = Box::new([0u8; HEAD_LEN]);
                loop {
                    match MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream, None).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                // why we choose to use futures::select!{} but tokio::select!{}?
                // the reason is that tokio::select!{} has bug in high concurrent network request.
                // but with futures::select!{}, some code may run slower caused by mutable reference required by futures::select!{}.
                // (to locate this bug takes me 4 days 😢
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        // tokio::spawn(async move {
        //     let mut buffer = Box::new([0u8; HEAD_LEN]);
        //     loop {
        //         match MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream).await {
        //             Ok(msg) => {
        //                 if let Err(e) = recv_sender.send(msg).await {
        //                     error!("send msg error: {:?}", e);
        //                     break;
        //                 }
        //             }
        //             Err(_) => {
        //                 break;
        //             }
        //         }
        //     }
        // });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub struct MsgIOTimeoutWrapper {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
    pub(self) timeout_channel: Option<MsgMpscReceiver>,
}

impl MsgIOTimeoutWrapper {
    pub(self) fn new(
        mut send_stream: SendStream,
        mut recv_stream: RecvStream,
        timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let ack_map = DashMap::new();

            let task1 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            match MsgIOUtil::send_msg(msg.clone(), &mut send_stream).await {
                                Ok(_) => {
                                    if skip_set.contains(&msg.typ()) || msg.typ() == Type::Ack {
                                        continue;
                                    }
                                    let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
                                    ack_map.insert(key, true);
                                    let timeout_sender = timeout_sender.clone();
                                    let ack_map = ack_map.clone();
                                    // todo change to single timer and priority queue
                                    tokio::spawn(async move {
                                        tokio::time::sleep(timeout).await;
                                        let flag = ack_map.get(&key);
                                        if let Some(_) = flag {
                                            _ = timeout_sender.send(msg).await;
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("send msg error: {:?}", e);
                                    break;
                                }
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                loop {
                    match MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream, None).await {
                        Ok(msg) => {
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload())
                                    .parse::<u64>()
                                    .unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
            timeout_channel: Some(timeout_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_channel.take().unwrap();
        (send, recv, timeout)
    }
}

pub struct MsgIOTlsServerTimeoutWrapper {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
    pub(self) timeout_receiver: Option<MsgMpscReceiver>,
}

impl MsgIOTlsServerTimeoutWrapper {
    pub(self) fn new(
        mut send_stream: WriteHalf<tls_server::TlsStream<TcpStream>>,
        mut recv_stream: ReadHalf<tls_server::TlsStream<TcpStream>>,
        timeout: Duration,
        idle_timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        let close_sender = send_sender.clone();
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let ack_map = DashMap::new();
            let timer = SharedTimer::new(timeout, async move {
                let mut msg = Msg::raw(0, 0, 0, &[]);
                msg.set_type(Type::Close);
                _ = close_sender.send(Arc::new(msg)).await;
            });
            let timer_setter = timer.setter();
            tokio::spawn(async move {
                timer.await;
            });

            let timer_setter1 = timer_setter.clone();
            let task1 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if msg.typ() == Type::Close {
                                _ = send_stream.shutdown().await;
                            }
                            timer_setter1
                                .set(tokio::time::Instant::now() + idle_timeout)
                                .await;
                            if let Err(e) =
                                MsgIOUtil::send_msg_server(msg.clone(), &mut send_stream).await
                            {
                                error!("send msg error: {:?}", e);
                                break;
                            } else {
                                if skip_set.contains(&msg.typ()) || msg.typ() == Type::Ack {
                                    continue;
                                }
                                let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, true);
                                let timeout_sender = timeout_sender.clone();
                                let ack_map = ack_map.clone();
                                // todo change to single timer and priority queue
                                tokio::spawn(async move {
                                    tokio::time::sleep(timeout).await;
                                    let flag = ack_map.get(&key);
                                    if let Some(_) = flag {
                                        _ = timeout_sender.send(msg).await;
                                    }
                                });
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let timer_setter2 = timer_setter;
            let task2 = async {
                loop {
                    match MsgIOUtil::recv_msg_server(&mut buffer, &mut recv_stream).await {
                        Ok(msg) => {
                            timer_setter2
                                .set(tokio::time::Instant::now() + idle_timeout)
                                .await;
                            if msg.typ() == Type::Ping {
                                continue;
                            }
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload())
                                    .parse::<u64>()
                                    .unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                futures::select! {
                    _ = task1 => {
                    },
                    _ = task2 => {
                    }
                    complete => {
                        break;
                    }
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
            timeout_receiver: Some(timeout_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_receiver.take().unwrap();
        (send, recv, timeout)
    }
}

pub(self) struct MsgIOTlsClientTimeoutWrapper {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
    pub(self) timeout_receiver: Option<MsgMpscReceiver>,
}

impl MsgIOTlsClientTimeoutWrapper {
    pub(self) fn new(
        mut send_stream: WriteHalf<tls_client::TlsStream<TcpStream>>,
        mut recv_stream: ReadHalf<tls_client::TlsStream<TcpStream>>,
        timeout: Duration,
        keep_alive_interval: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        let tick_sender = send_sender.clone();
        tokio::spawn(async move {
            let ack_map0 = Arc::new(DashMap::new());
            let mut ticker = tokio::time::interval(keep_alive_interval);

            let ack_map = ack_map0.clone();
            let task1 = async move {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) =
                                MsgIOUtil::send_msg_client(msg.clone(), &mut send_stream).await
                            {
                                error!("send msg error: {:?}", e);
                                break;
                            } else {
                                if skip_set.contains(&msg.typ()) || msg.typ() == Type::Ack {
                                    continue;
                                }
                                let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, true);
                                let timeout_sender = timeout_sender.clone();
                                let ack_map = ack_map.clone();
                                // todo change to single timer and priority queue
                                tokio::spawn(async move {
                                    tokio::time::sleep(timeout).await;
                                    let flag = ack_map.get(&key);
                                    if let Some(_) = flag {
                                        _ = timeout_sender.send(msg).await;
                                    }
                                });
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let ack_map = ack_map0;
            let task2 = async move {
                let mut buffer = Box::new([0u8; HEAD_LEN]);
                loop {
                    match MsgIOUtil::recv_msg_client(&mut buffer, &mut recv_stream).await {
                        Ok(msg) => {
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload())
                                    .parse::<u64>()
                                    .unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task3 = async move {
                loop {
                    ticker.tick().await;
                    let msg = Arc::new(Msg::ping(0, 0, 0));
                    if let Err(e) = tick_sender.send(msg).await {
                        error!("send msg error: {:?}", e);
                        break;
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2, task3);

            loop {
                select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    _ = task3 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
            timeout_receiver: Some(timeout_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_receiver.take().unwrap();
        (send, recv, timeout)
    }
}

pub struct TinyMsgIOUtil {}

impl TinyMsgIOUtil {
    pub async fn send_msg(msg: &TinyMsg, send_stream: &mut SendStream) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            error!("write stream error: {:?}", e);
            _ = send_stream.shutdown().await;
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        Ok(())
    }

    pub async fn recv_msg(recv_stream: &mut RecvStream) -> Result<TinyMsg> {
        let mut len_buf: [u8; 2] = [0u8; 2];
        match recv_stream.read_exact(&mut len_buf[..]).await {
            Ok(_) => {}
            Err(e) => {
                return match e {
                    ReadExactError::FinishedEarly => {
                        info!("stream finished.");
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "stream finished.".to_string()
                        )))
                    }
                    ReadExactError::ReadError(e) => {
                        debug!("read stream error: {:?}", e);
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                };
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = TinyMsg::pre_alloc(len);
        match recv_stream.read_exact(msg.payload_mut()).await {
            Ok(_) => {}
            Err(e) => {
                return match e {
                    ReadExactError::FinishedEarly => {
                        info!("stream finished.");
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "stream finished.".to_string()
                        )))
                    }
                    ReadExactError::ReadError(e) => {
                        debug!("read stream error: {:?}", e);
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                };
            }
        };
        Ok(msg)
    }

    pub async fn send_msg_client(
        msg: &TinyMsg,
        send_stream: &mut WriteHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        Ok(())
    }

    pub async fn recv_msg_client(
        recv_stream: &mut ReadHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<TinyMsg> {
        let mut len_buf: [u8; 2] = [0u8; 2];
        match recv_stream.read_exact(&mut len_buf[..]).await {
            Ok(_) => {}
            Err(e) => {
                debug!("read stream error: {:?}", e);
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = TinyMsg::pre_alloc(len);
        match recv_stream.read_exact(msg.payload_mut()).await {
            Ok(_) => {}
            Err(e) => {
                debug!("read stream error: {:?}", e);
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        Ok(msg)
    }

    pub async fn send_msg_server(
        msg: &TinyMsg,
        send_stream: &mut WriteHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        Ok(())
    }

    pub async fn recv_msg_server(
        recv_stream: &mut ReadHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<TinyMsg> {
        let mut len_buf: [u8; 2] = [0u8; 2];
        match recv_stream.read_exact(&mut len_buf[..]).await {
            Ok(_) => {}
            Err(e) => {
                debug!("read stream error: {:?}", e);
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = TinyMsg::pre_alloc(len);
        match recv_stream.read_exact(msg.payload_mut()).await {
            Ok(_) => {}
            Err(e) => {
                debug!("read stream error: {:?}", e);
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        Ok(msg)
    }
}

pub struct ReqwestMsgIOUtil {}

impl ReqwestMsgIOUtil {
    #[inline(always)]
    pub async fn send_msg(msg: &ReqwestMsg, send_stream: &mut SendStream) -> Result<()> {
        #[cfg(feature = "pre-check")]
        if pre_check(msg.as_slice()) != msg.as_slice().len() {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "invalid message.".to_string()
            )));
        }
        if let Err(e) = send_stream.write_all(&MSG_DELIMITER).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        Ok(())
    }

    #[async_recursion]
    pub async fn recv_msg<'a: 'async_recursion, 'b: 'async_recursion>(
        recv_stream: &mut RecvStream,
        mut external_source: Option<&'b [u8]>,
    ) -> Result<ReqwestMsg> {
        #[cfg(feature = "pre-check")]
        {
            let mut from = 0;
            let mut delimiter_buf = [0u8; 4];
            let mut loss = 0;
            loop {
                match read_buffer(recv_stream, external_source, &mut delimiter_buf[from..]).await {
                    Ok(external_source0) => {
                        external_source = external_source0;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                if delimiter_buf[0] != MSG_DELIMITER[0] {
                    loss += 1;
                    debug!("invalid message detected[1].");
                    delimiter_buf[0] = delimiter_buf[1];
                    delimiter_buf[1] = delimiter_buf[2];
                    delimiter_buf[2] = delimiter_buf[3];
                    from = 3;
                    continue;
                } else if delimiter_buf[1] != MSG_DELIMITER[1] {
                    loss += 2;
                    debug!("invalid message detected[2].");
                    delimiter_buf[0] = delimiter_buf[2];
                    delimiter_buf[1] = delimiter_buf[3];
                    from = 2;
                    continue;
                } else if delimiter_buf[2] != MSG_DELIMITER[2] {
                    loss += 3;
                    debug!("invalid message detected[3].");
                    delimiter_buf[0] = delimiter_buf[3];
                    from = 1;
                    continue;
                } else if delimiter_buf[3] != MSG_DELIMITER[3] {
                    loss += 4;
                    debug!("invalid message detected[4].");
                    from = 0;
                } else {
                    break;
                }
            }
            if loss != 0 {
                error!(
                    "{} message loss {} bytes detected.",
                    recv_stream.id().0,
                    loss
                );
            }
        }
        let mut len_buf: [u8; 2] = [0u8; 2];
        match read_buffer(recv_stream, external_source, &mut len_buf).await {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = ReqwestMsg::pre_alloc(len);
        match read_buffer(recv_stream, external_source, &mut msg.body_mut()).await {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(feature = "pre-check")]
        {
            let index = pre_check(msg.as_slice());
            if index != msg.as_slice().len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&msg.as_slice()[index..]);
                let res = ReqwestMsgIOUtil::recv_msg(recv_stream, Some(&external)).await;
                return res;
            }
        }
        Ok(msg)
    }
}

pub struct ReqwestMsgIOWrapper {
    pub(self) send_channel: Option<tokio::sync::mpsc::Sender<ReqwestMsg>>,
    pub(self) recv_channel: Option<tokio::sync::mpsc::Receiver<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapper {
    pub fn new(mut send_stream: SendStream, mut recv_stream: RecvStream) -> Self {
        let (send_sender, mut send_receiver): (
            tokio::sync::mpsc::Sender<ReqwestMsg>,
            tokio::sync::mpsc::Receiver<ReqwestMsg>,
        ) = tokio::sync::mpsc::channel(16384);
        let (recv_sender, recv_receiver): (
            tokio::sync::mpsc::Sender<ReqwestMsg>,
            tokio::sync::mpsc::Receiver<ReqwestMsg>,
        ) = tokio::sync::mpsc::channel(16384);
        #[cfg(not(feature = "no-select"))]
        tokio::spawn(async move {
            let task1 = async {
                loop {
                    match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {}", e.to_string());
                                break;
                            }
                        }
                        Err(_) => {
                            _ = recv_stream.stop(0u32.into());
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await
                            {
                                error!("send msg error: {}", e.to_string());
                                break;
                            }
                        }
                        None => {
                            _ = send_stream.finish().await;
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        #[cfg(feature = "no-select")]
        tokio::spawn(async move {
            loop {
                match send_receiver.recv().await {
                    Some(msg) => {
                        if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await {
                            error!("send msg error: {:?}", e);
                            break;
                        }
                    }
                    None => {
                        _ = send_stream.finish().await;
                        break;
                    }
                }
            }
        });
        #[cfg(feature = "no-select")]
        tokio::spawn(async move {
            loop {
                match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None, 1).await {
                    Ok(msg) => {
                        if let Err(e) = recv_sender.send(msg).await {
                            error!("send msg error: {}", e.to_string());
                            break;
                        }
                    }
                    Err(_) => {
                        _ = recv_stream.stop(0u32.into());
                        break;
                    }
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn channels(
        &mut self,
    ) -> (
        tokio::sync::mpsc::Sender<ReqwestMsg>,
        tokio::sync::mpsc::Receiver<ReqwestMsg>,
    ) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

#[cfg(test)]
mod test {
    use crate::net::pre_check;

    #[test]
    fn test() {
        let arr = vec![25, 255, 255, 255, 255, 25, 255];
        println!("{}", pre_check(&arr[..]));
    }
}
