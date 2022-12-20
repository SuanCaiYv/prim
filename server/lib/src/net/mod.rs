pub mod client;
pub mod server;

use ahash::AHashSet;
use std::{sync::Arc, time::Duration};

use crate::{
    entity::{Head, Msg, Type, EXTENSION_THRESHOLD, HEAD_LEN, PAYLOAD_THRESHOLD},
    Result,
};
use anyhow::anyhow;
use dashmap::DashMap;
use quinn::{ReadExactError, RecvStream, SendStream};
use tracing::{debug, info};
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
        let readable = recv_stream.read_exact(&mut buffer[..]).await;
        match readable {
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
                }
            }
        }
        let mut head = Head::from(&buffer[..]);
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(&mut head);
        let size = recv_stream
            .read_exact(&mut (msg.as_mut_slice()[HEAD_LEN..]))
            .await;
        match size {
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
                }
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
        let res = send_stream.write_all(msg.as_slice()).await;
        if let Err(e) = res {
            send_stream.finish().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }
}

pub(self) struct MsgIOTimeoutUtil {
    ack_map: AckMap,
    buffer: Box<[u8; HEAD_LEN]>,
    timeout: Duration,
    timeout_channel_sender: InnerSender,
    timeout_channel_receiver: Option<OuterReceiver>,
    io_streams: (SendStream, RecvStream),
    skip_set: AHashSet<Type>,
}

impl MsgIOTimeoutUtil {
    pub(self) fn new(
        io_streams: (SendStream, RecvStream),
        timeout: Duration,
        channel_buffer_size: usize,
        skip_set: Option<AHashSet<Type>>,
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
            match msg.typ() {
                Type::Ack => {
                    let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>()?;
                    let key = timestamp % TIMEOUT_WHEEL_SIZE;
                    self.ack_map.insert(key, false);
                }
                _ => {
                    return Ok(msg);
                }
            }
        }
    }

    pub(self) fn timeout_channel_receiver(&mut self) -> OuterReceiver {
        self.timeout_channel_receiver.take().unwrap()
    }
}
