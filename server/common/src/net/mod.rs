pub mod client;
pub mod server;

use crate::entity::{Msg, HEAD_LEN};
use crate::Result;
use anyhow::anyhow;
use quinn::{ReadExactError, RecvStream, SendStream};
use std::sync::Arc;
use tracing::{info, warn};

pub type LenBuffer = [u8; 4];
/// the direction is relative to the stream task.
pub type InnerSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type InnerReceiver = async_channel::Receiver<Arc<Msg>>;
pub type OuterSender = async_channel::Sender<Arc<Msg>>;
pub type OuterReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;
pub const BODY_SIZE: usize = (1 << 12) + 64;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];
pub const CLUSTER_HASH_SIZE: u64 = u16::MAX as u64;

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
        Ok(())
    }
}
