pub mod server;
pub mod client;

use std::sync::Arc;

use crate::entity::{Msg, HEAD_LEN};
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use quinn::{NewConnection, ReadExactError, RecvStream, SendStream};

use tracing::{info, warn};

pub type Result<T> = anyhow::Result<T>;
pub type LenBuffer = [u8; 4];
/// the direction is relative to the stream task.
pub type InnerSender = tokio::sync::mpsc::Sender<Arc<Msg>>;
pub type InnerReceiver = async_channel::Receiver<Arc<Msg>>;
pub type OuterSender = async_channel::Sender<Arc<Msg>>;
pub type OuterReceiver = tokio::sync::mpsc::Receiver<Arc<Msg>>;
pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);
pub type ConnectionTaskGenerator =
    Box<dyn (Fn(NewConnection) -> Box<dyn ConnectionTask>) + Send + Sync + 'static>;
pub const BODY_SIZE: usize = 1 << 16;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Result<&T> {
        let parameter = self.0.get(std::any::type_name::<T>());
        if parameter.is_none() {
            Err(anyhow!("parameter not found"))
        } else {
            let parameter = parameter.unwrap();
            let parameter = parameter.as_any().downcast_ref::<T>();
            if parameter.is_none() {
                Err(anyhow!("parameter type mismatch"))
            } else {
                Ok(parameter.unwrap())
            }
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Result<&mut T> {
        let parameter = self.0.get_mut(std::any::type_name::<T>());
        if parameter.is_none() {
            Err(anyhow!("parameter not found"))
        } else {
            let parameter = parameter.unwrap();
            let parameter = parameter.as_mut_any().downcast_mut::<T>();
            if parameter.is_none() {
                Err(anyhow!("parameter type mismatch"))
            } else {
                Ok(parameter.unwrap())
            }
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub struct HandlerParameters {
    #[allow(unused)]
    pub buffer: LenBuffer,
    /// in/out streams interacting with quic
    #[allow(unused)]
    pub streams: (SendStream, RecvStream),
    /// inner streams interacting with other tasks
    /// why tokio? cause this direction's model is multi-sender and single-receiver
    /// why async-channel? cause this direction's model is single-sender multi-receiver
    pub inner_streams: (InnerSender, InnerReceiver),
    #[allow(unused)]
    pub generic_parameters: GenericParameterMap,
}

#[async_trait]
pub trait ConnectionTask: Send + Sync + 'static {
    /// this method will run in a new tokio task.
    async fn handle(mut self: Box<Self>) -> Result<()>;
}

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
        msg.update_payload_length(payload_size);
        msg.update_extension_length(extension_size);
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
        let res = send.write_all(msg.as_bytes().as_slice()).await;
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
