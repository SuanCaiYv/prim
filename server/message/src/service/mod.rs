use std::sync::{atomic::AtomicUsize, Arc};

use ahash::AHashMap;
use anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use lazy_static::lazy_static;
use lib::{net::GenericParameter, Result};
use lib_net_tokio::net::{MsgSender, ReqwestOperatorManager};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};
use tracing::error;

use self::handler::io_task;
use crate::{service::handler::IOTaskReceiver, CPU_NUM};

pub(crate) mod handler;
pub(crate) mod server;

pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, MsgSender>>);
pub(crate) struct MsgloggerClient(pub(crate) tokio::net::UnixStream);

lazy_static! {
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
    pub(self) static ref SEQNUM_CLIENT_MAP: Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>> =
        Arc::new(RwLock::new(AHashMap::new()));
    pub(self) static ref CLIENT_INDEX: AtomicUsize = AtomicUsize::new(0);
}

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_seqnum_client_map() -> Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>> {
    SEQNUM_CLIENT_MAP.clone()
}

pub(crate) async fn get_msglogger_client() -> Result<MsgloggerClient> {
    let index = CLIENT_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let index = index % unsafe { CPU_NUM };
    let addr = format!("/tmp/msglogger-{}.sock", index);
    let stream = tokio::net::UnixStream::connect(addr).await?;
    Ok(MsgloggerClient(stream))
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for MsgloggerClient {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClientConnectionMap {
    pub(crate) fn get<'a>(&'a self, id: &u64) -> Option<Ref<'a, u64, MsgSender>> {
        self.0.get(id)
    }

    pub(crate) fn insert(&self, id: u64, sender: MsgSender) {
        self.0.insert(id, sender);
    }
}

impl MsgloggerClient {
    pub(crate) async fn log(&mut self, msg: &[u8]) -> Result<()> {
        self.0.write_all(msg).await?;
        let a = self.0.read_u8().await?;
        let b = self.0.read_u8().await?;
        if a != b'o' || b != b'k' {
            error!("msglogger client log error");
            return Err(anyhow!("msglogger client log error"));
        }
        Ok(())
    }
}

pub(crate) async fn start(io_task_receiver: IOTaskReceiver) -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = io_task(io_task_receiver).await {
            error!("io task error: {}", e);
        }
    });
    server::Server::run().await?;
    Ok(())
}
