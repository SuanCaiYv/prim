pub mod client;
pub mod server;

use ahash::AHashSet;
use byteorder::{BigEndian, ByteOrder};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream, select,
};
use tokio_rustls::{client as tls_client, server as tls_server};

use crate::{
    entity::{Head, Msg, TinyMsg, Type, EXTENSION_THRESHOLD, HEAD_LEN, PAYLOAD_THRESHOLD},
    Result,
};
use anyhow::anyhow;
use dashmap::DashMap;
use quinn::{ReadExactError, RecvStream, SendStream};
use tracing::{debug, info, error};

/// the direction is relative to the stream task.
///
/// why tokio? cause this direction's model is multi-sender and single-receiver
///
/// why async-channel? cause this direction's model is single-sender multi-receiver
pub type InnerSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type InnerReceiver = async_channel::Receiver<Arc<Msg>>;
pub type OuterSender = async_channel::Sender<Arc<Msg>>;
pub type OuterReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;

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
    pub(self) auth_channel: Option<tokio::sync::mpsc::Receiver<Arc<Msg>>>,
    pub(self) send_channel: Option<tokio::sync::mpsc::Sender<Arc<Msg>>>,
    pub(self) recv_channel: Option<tokio::sync::mpsc::Receiver<Arc<Msg>>>,
}

impl MsgIOWrapper {
    pub fn new(auth_stream: RecvStream, send_stream: SendStream, recv_stream: RecvStream) -> Self {
        let auth = tokio::sync::mpsc::channel(64);
        let send = tokio::sync::mpsc::channel(64);
        let recv = tokio::sync::mpsc::channel(64);
        let (auth_sender, auth_receiver) = auth;
        let (send_sender, send_receiver) = send;
        let (recv_sender, recv_receiver) = recv;
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

    pub fn channels(&mut self) -> (tokio::sync::mpsc::Receiver<Arc<Msg>>, tokio::sync::mpsc::Sender<Arc<Msg>>, tokio::sync::mpsc::Receiver<Arc<Msg>>) {
        let auth = self.auth_channel.take().unwrap();
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (auth, send, recv)
    }
}

pub(self) struct MsgIOTimeoutWrapper {
    ack_map: AckMap,
    buffer: Box<[u8; HEAD_LEN]>,
    timeout: Duration,
    timeout_channel_sender: InnerSender,
    timeout_channel_receiver: Option<OuterReceiver>,
    io_streams: (SendStream, RecvStream),
    // the set of message types that should not be timeout.
    skip_set: AHashSet<Type>,
    // whether the upstream wants the ack backed.
    ack_needed: bool,
}

impl MsgIOTimeoutWrapper {
    pub(self) fn new(
        io_streams: (SendStream, RecvStream),
        timeout: Duration,
        skip_set: Option<AHashSet<Type>>,
        ack_needed: bool,
    ) -> Self {
        let (timeout_channel_sender, timeout_channel_receiver) = tokio::sync::mpsc::channel(64);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        Self {
            ack_map: AckMap::new(DashMap::new()),
            buffer: Box::new([0; HEAD_LEN]),
            timeout,
            timeout_channel_sender,
            timeout_channel_receiver: Some(timeout_channel_receiver),
            io_streams,
            skip_set,
            ack_needed,
        }
    }

    pub(self) async fn send_msg(&mut self, msg: Arc<Msg>) -> Result<()> {
        MsgIOUtil::send_msg(msg.clone(), &mut self.io_streams.0).await?;
        if self.skip_set.contains(&msg.typ()) {
            return Ok(());
        }
        let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
        self.ack_map.insert(key, true);
        let timeout_channel_sender = self.timeout_channel_sender.clone();
        let ack_map = self.ack_map.clone();
        let timeout = self.timeout;
        // todo change to single timer and priority queue
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            let flag = ack_map.get(&key);
            if let Some(_) = flag {
                _ = timeout_channel_sender.send(msg).await;
            }
        });
        Ok(())
    }

    pub(self) async fn recv_msg(&mut self) -> Result<Arc<Msg>> {
        loop {
            let msg = MsgIOUtil::recv_msg(&mut self.buffer, &mut self.io_streams.1).await?;
            if msg.typ() == Type::Ack {
                let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>()?;
                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                self.ack_map.insert(key, false);
                if !self.ack_needed {
                    continue;
                }
            }
            break Ok(msg);
        }
    }

    pub(self) fn timeout_channel_receiver(&mut self) -> OuterReceiver {
        self.timeout_channel_receiver.take().unwrap()
    }
}

pub(self) struct MsgIOTlsTimeoutWrapper {
    ack_map: AckMap,
    buffer: Box<[u8; HEAD_LEN]>,
    timeout: Duration,
    timeout_channel_sender: InnerSender,
    timeout_channel_receiver: Option<OuterReceiver>,
    io_streams: (
        WriteHalf<tls_client::TlsStream<TcpStream>>,
        ReadHalf<tls_client::TlsStream<TcpStream>>,
    ),
    skip_set: AHashSet<Type>,
    ack_needed: bool,
}

impl MsgIOTlsTimeoutWrapper {
    pub(self) fn new(
        io_streams: (
            WriteHalf<tls_client::TlsStream<TcpStream>>,
            ReadHalf<tls_client::TlsStream<TcpStream>>,
        ),
        timeout: Duration,
        channel_buffer_size: usize,
        skip_set: Option<AHashSet<Type>>,
        ack_needed: bool,
    ) -> Self {
        let (timeout_channel_sender, timeout_channel_receiver) =
            tokio::sync::mpsc::channel(channel_buffer_size);
        let skip_set = match skip_set {
            Some(v) => v,
            None => AHashSet::new(),
        };
        Self {
            ack_map: AckMap::new(DashMap::new()),
            buffer: Box::new([0; HEAD_LEN]),
            timeout,
            timeout_channel_sender,
            timeout_channel_receiver: Some(timeout_channel_receiver),
            io_streams,
            skip_set,
            ack_needed,
        }
    }

    pub(self) async fn send_msg(&mut self, msg: Arc<Msg>) -> Result<()> {
        MsgIOUtil::send_msg_client(msg.clone(), &mut self.io_streams.0).await?;
        if self.skip_set.contains(&msg.typ()) {
            return Ok(());
        }
        let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
        self.ack_map.insert(key, true);
        let timeout_channel_sender = self.timeout_channel_sender.clone();
        let ack_map = self.ack_map.clone();
        let timeout = self.timeout;
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            let flag = ack_map.get(&key);
            if let Some(_) = flag {
                _ = timeout_channel_sender.send(msg).await;
            }
        });
        Ok(())
    }

    pub(self) async fn recv_msg(&mut self) -> Result<Arc<Msg>> {
        loop {
            let msg = MsgIOUtil::recv_msg_client(&mut self.buffer, &mut self.io_streams.1).await?;
            if msg.typ() == Type::Ack {
                let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>()?;
                let key = timestamp % TIMEOUT_WHEEL_SIZE;
                self.ack_map.insert(key, false);
                if !self.ack_needed {
                    continue;
                }
            }
            break Ok(msg);
        }
    }

    pub(self) fn timeout_channel_receiver(&mut self) -> OuterReceiver {
        self.timeout_channel_receiver.take().unwrap()
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
