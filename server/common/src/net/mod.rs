pub mod client;
pub mod server;

use crate::entity::{Msg, Type, HEAD_LEN};
use crate::Result;
use anyhow::anyhow;
use dashmap::DashMap;
use delay_timer::prelude::{DelayTimerBuilder, TaskBuilder};
use quinn::{ReadExactError, RecvStream, SendStream};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use tracing::debug;

pub type LenBuffer = [u8; 4];
/// the direction is relative to the stream task.
///
/// why tokio? cause this direction's model is multi-sender and single-receiver
///
/// why async-channel? cause this direction's model is single-sender multi-receiver
pub type InnerSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type InnerReceiver = async_channel::Receiver<Arc<Msg>>;
pub type OuterSender = async_channel::Sender<Arc<Msg>>;
pub type OuterReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;
type AckMap = Arc<DashMap<u64, bool>>;
pub const BODY_SIZE: usize = (1 << 12) + 64;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
const TIMEOUT_WHEEL_SIZE: u64 = 4096;

pub struct MsgIO;

impl MsgIO {
    /// the only error returned should cause the stream crashed.
    ///
    /// the purpose using [`std::sync::Arc`] is to reduce unnecessary clone.
    #[allow(unused)]
    #[inline]
    pub async fn read_msg(buffer: &mut LenBuffer, recv: &mut RecvStream) -> Result<Arc<Msg>> {
        let readable = recv.read_exact(&mut buffer[..]).await;
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
                        warn!("read stream error: {:?}", e);
                        Err(anyhow!(crate::error::CrashError::ShouldCrash(
                            "read stream error.".to_string()
                        )))
                    }
                }
            }
        }
        let extension_size = Msg::read_u16(&buffer[0..2]);
        let payload_size = Msg::read_u16(&buffer[2..4]);
        if (payload_size + extension_size) as usize > BODY_SIZE {
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(payload_size, extension_size);
        msg.set_payload_length(payload_size);
        msg.set_extension_length(extension_size);
        let size = recv
            .read_exact(
                &mut (msg.as_mut_slice()
                    [4..(HEAD_LEN + extension_size as usize + payload_size as usize)]),
            )
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
                        warn!("read stream error: {:?}", e);
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
    pub async fn write_msg(msg: Arc<Msg>, send: &mut SendStream) -> Result<()> {
        let res = send.write_all(msg.as_slice()).await;
        if let Err(e) = res {
            send.finish().await;
            warn!("write stream error: {:?}", e);
            return Err(anyhow!(crate::error::CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }
}

pub struct MsgIOTimeOut {
    read_channel: OuterReceiver,
    write_channel: InnerSender,
    timeout_channel: OuterReceiver,
}

impl MsgIOTimeOut {
    /// the timeout duration should less than 4000ms, otherwise there may be weird behavior.
    pub fn new(io_streams: (SendStream, RecvStream), write_timeout: Duration) -> Self {
        let read_channel = tokio::sync::mpsc::channel(32);
        let write_channel = tokio::sync::mpsc::channel(32);
        let timeout_channel = tokio::sync::mpsc::channel(32);
        let ack_map = AckMap::new(DashMap::new());
        tokio::spawn(Self::read_msg(
            io_streams.1,
            ack_map.clone(),
            read_channel.0,
        ));
        tokio::spawn(Self::write_msg(
            io_streams.0,
            ack_map.clone(),
            write_timeout,
            write_channel.1,
            timeout_channel.0,
        ));
        Self {
            read_channel: read_channel.1,
            write_channel: write_channel.0,
            timeout_channel: timeout_channel.1,
        }
    }

    pub fn channels(self) -> (OuterReceiver, InnerSender, OuterReceiver) {
        let Self {
            read_channel,
            write_channel,
            timeout_channel,
        } = self;
        (read_channel, write_channel, timeout_channel)
    }

    async fn read_msg(
        mut recv: RecvStream,
        ack_map: AckMap,
        read_channel: InnerSender,
    ) -> Result<()> {
        let mut buffer = [0u8; 4];
        loop {
            let msg = MsgIO::read_msg(&mut buffer, &mut recv).await?;
            match msg.typ() {
                Type::Ack => {
                    let timestamp = String::from_utf8_lossy(msg.payload()).parse::<u64>()?;
                    let key = timestamp % TIMEOUT_WHEEL_SIZE;
                    ack_map.insert(key, false);
                }
                _ => {
                    read_channel.send(msg).await?;
                }
            }
        }
    }

    async fn write_msg(
        mut send: SendStream,
        ack_map: AckMap,
        write_timeout: Duration,
        mut write_channel: OuterReceiver,
        timeout_channel: InnerSender,
    ) -> Result<()> {
        let timer = DelayTimerBuilder::default()
            .tokio_runtime_by_default()
            .build();
        loop {
            let msg = write_channel.recv().await;
            if msg.is_none() {
                // channel closed.
                break;
            }
            let msg = msg.unwrap();
            MsgIO::write_msg(msg.clone(), &mut send).await?;
            let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
            ack_map.insert(key, true);
            let timeout_channel = timeout_channel.clone();
            let ack_map = ack_map.clone();
            let task = TaskBuilder::default()
                .set_task_id(msg.timestamp())
                .set_frequency_once_by_seconds(write_timeout.as_secs())
                .set_maximum_parallel_runnable_num(1)
                .spawn_async_routine(move || {
                    let timeout_channel = timeout_channel.clone();
                    let msg = msg.clone();
                    let key = msg.timestamp() % TIMEOUT_WHEEL_SIZE;
                    let flag = ack_map.get(&key);
                    let mut sent = true;
                    if let Some(flag) = flag {
                        sent = !(*flag);
                    }
                    async move {
                        if sent {
                            let _ = timeout_channel.send(msg).await;
                        }
                    }
                })?;
            timer.insert_task(task)?;
        }
        Ok(())
    }
}
