pub mod client;
pub mod server;

use ahash::{AHashMap, AHashSet};
use byteorder::{BigEndian, ByteOrder};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    select,
};
use tokio_rustls::{client as tls_client, server as tls_server};

use crate::{
    entity::{Head, Msg, TinyMsg, Type, EXTENSION_THRESHOLD, HEAD_LEN, PAYLOAD_THRESHOLD},
    Result,
};
use anyhow::anyhow;
use dashmap::DashMap;
use quinn::{ReadExactError, RecvStream, SendStream};
use tracing::{debug, error, info};

pub type MsgSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type MsgReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;

/// the direction is relative to the stream task.
///
/// why tokio? cause this direction's model is multi-sender and single-receiver
///
/// why async-channel? cause this direction's model is single-sender multi-receiver
pub type InnerSender = MsgSender;
pub type InnerReceiver = async_channel::Receiver<Arc<Msg>>;
pub type OuterSender = async_channel::Sender<Arc<Msg>>;
pub type OuterReceiver = MsgReceiver;

pub(self) type AckMap = Arc<DashMap<u64, bool>>;

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
pub(self) const TIMEOUT_WHEEL_SIZE: u64 = 4096;

pub(self) struct MsgIOUtil;

impl MsgIOUtil {
    /// the only error returned should cause the stream crashed.
    ///
    /// the purpose using [`std::sync::Arc`] is to reduce unnecessary clone.
    #[allow(unused)]
    #[inline]
    pub(self) async fn recv_msg(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut RecvStream,
    ) -> Result<Arc<Msg>> {
        match recv_stream.read_exact(&mut buffer[..]).await {
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
        }
        debug!("read msg: {}", msg);
        Ok(Arc::new(msg))
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn recv_msg_server(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut ReadHalf<tls_server::TlsStream<TcpStream>>,
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
    pub(self) auth_channel: Option<MsgReceiver>,
    pub(self) send_channel: Option<MsgSender>,
    pub(self) recv_channel: Option<MsgReceiver>,
}

impl MsgIOWrapper {
    pub fn new(auth_stream: RecvStream, send_stream: SendStream, recv_stream: RecvStream) -> Self {
        // actually channel buffer size set to 1 is more intuitive.
        let (auth_sender, auth_receiver): (MsgSender, MsgReceiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, send_receiver): (MsgSender, MsgReceiver) = tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver): (MsgSender, MsgReceiver) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            loop {
                select! {
                    msg = MsgIOUtil::recv_msg(&mut buffer, &mut auth_stream) => {
                        if let Ok(msg) = msg {
                            if let Err(e) = auth_sender.send(msg).await {
                                error!("send auth msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    },
                    msg = send_receiver.recv() => {
                        if let Some(msg) = msg {
                            if let Err(e) = MsgIOUtil::send_msg(msg, &mut send_stream).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    },
                    msg = MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream) => {
                        if let Ok(msg) = msg {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });
        Self {
            auth_channel: Some(auth_receiver),
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgReceiver, MsgSender, MsgReceiver) {
        let auth = self.auth_channel.take().unwrap();
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (auth, send, recv)
    }
}

pub(self) struct MsgIOTimeoutWrapper {
    pub(self) auth_channel: Option<MsgReceiver>,
    pub(self) send_channel: Option<MsgSender>,
    pub(self) recv_channel: Option<MsgReceiver>,
    pub(self) timeout_channel: Option<MsgReceiver>,
}

impl MsgIOTimeoutWrapper {
    pub(self) fn new(
        auth_stream: RecvStream,
        send_stream: SendStream,
        recv_stream: RecvStream,
        timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (auth_sender, auth_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, send_receiver) = tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let mut ack_map = AHashMap::new();
            loop {
                select! {
                    msg = MsgIOUtil::recv_msg(&mut buffer, &mut auth_stream) => {
                        if let Ok(msg) = msg {
                            if let Err(e) = auth_sender.send(msg).await {
                                error!("send auth msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    },
                    msg = send_receiver.recv() => {
                        if let Some(msg) = msg {
                            if let Err(e) = MsgIOUtil::send_msg(msg, &mut send_stream).await {
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
                        } else {
                            break;
                        }
                    },
                    msg = MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream) => {
                        if let Ok(msg) = msg {
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>().unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });
        Self {
            auth_channel: Some(auth_receiver),
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
            timeout_channel: Some(timeout_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgReceiver, MsgSender, MsgReceiver, MsgReceiver) {
        let auth = self.auth_channel.take().unwrap();
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_channel.take().unwrap();
        (auth, send, recv, timeout)
    }
}

pub(self) struct MsgIOClientTimeoutWrapper {
    pub(self) send_channel: Option<MsgSender>,
    pub(self) recv_channel: Option<MsgReceiver>,
    pub(self) timeout_receiver: Option<MsgReceiver>,
}

impl MsgIOClientTimeoutWrapper {
    pub(self) fn new(
        send_stream: WriteHalf<tls_client::TlsStream<TcpStream>>,
        recv_stream: ReadHalf<tls_client::TlsStream<TcpStream>>,
        timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, send_receiver) = tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let mut ack_map = AHashMap::new();
            loop {
                select! {
                    msg = send_receiver.recv() => {
                        if let Some(msg) = msg {
                            if let Err(e) = MsgIOUtil::send_msg_client(msg, &mut send_stream).await {
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
                        } else {
                            break;
                        }
                    },
                    msg = MsgIOUtil::recv_msg_client(&mut buffer, &mut recv_stream) => {
                        if let Ok(msg) = msg {
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>().unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
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

    pub fn channels(&mut self) -> (MsgSender, MsgReceiver, MsgReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_receiver.take().unwrap();
        (send, recv, timeout)
    }
}

pub(self) struct MsgIOServerTimeoutWrapper {
    pub(self) send_channel: Option<MsgSender>,
    pub(self) recv_channel: Option<MsgReceiver>,
    pub(self) timeout_receiver: Option<MsgReceiver>,
}

impl MsgIOServerTimeoutWrapper {
    pub(self) fn new(
        send_stream: WriteHalf<tls_server::TlsStream<TcpStream>>,
        recv_stream: ReadHalf<tls_server::TlsStream<TcpStream>>,
        timeout: Duration,
        idle_timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
    ) -> Self {
        let (timeout_sender, timeout_receiver) = tokio::sync::mpsc::channel(64);
        let (send_sender, send_receiver) = tokio::sync::mpsc::channel(64);
        let (recv_sender, recv_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        let timer = tokio::time::sleep(idle_timeout);
        tokio::pin!(timer);
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let mut ack_map = AHashMap::new();
            loop {
                select! {
                    msg = send_receiver.recv() => {
                        if let Some(msg) = msg {
                            timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            if let Err(e) = MsgIOUtil::send_msg_server(msg, &mut send_stream).await {
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
                        } else {
                            break;
                        }
                    },
                    msg = MsgIOUtil::recv_msg_server(&mut buffer, &mut recv_stream) => {
                        if let Ok(msg) = msg {
                            timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                            if msg.typ() == Type::Ping {
                                continue;
                            }
                            if msg.typ() == Type::Ack {
                                let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>().unwrap_or(0);
                                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                                ack_map.insert(key, false);
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    _ = &mut timer => {
                        error!("connection idle timeout.");
                        break;
                    },
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
            timeout_receiver: Some(timeout_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgSender, MsgReceiver, MsgReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        let timeout = self.timeout_receiver.take().unwrap();
        (send, recv, timeout)
    }
}

pub(self) struct TinyMsgIOUtil {}

impl TinyMsgIOUtil {
    pub async fn send_msg(msg: &TinyMsg, send_stream: &mut SendStream) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
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
                return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        Ok(msg)
    }
}
