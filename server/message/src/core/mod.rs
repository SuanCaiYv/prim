use async_channel::{Sender, Receiver};
use quinn::{RecvStream, SendStream};
use std::sync::Arc;

use dashmap::DashMap;
use structopt::lazy_static::lazy_static;

use tonic::async_trait;

use crate::cache::redis_ops::RedisOps;
use crate::entity::{msg, HEAD_LEN};
pub(self) mod handler;
mod mock;
pub(self) mod server;

pub(self) const BODY_BUF_LENGTH: usize = 1 << 16;
pub(self) const ALPN_PRIM: &[&[u8]] = &[b"prim"];

pub(crate) type Result<T> = anyhow::Result<T>;

/// use Arc + ConcurrentMap + Clone to share state between Tasks
pub(self) type ConnectionMap = Arc<DashMap<u64, Sender<msg::Msg>>>;
pub(self) type StatusMap = Arc<DashMap<u64, u64>>;

lazy_static! {
    static ref CONNECTION_MAP: ConnectionMap = Arc::new(DashMap::new());
    static ref STATUS_MAP: StatusMap = Arc::new(DashMap::new());
}

pub(self) struct Buffer {
    #[allow(unused)]
    head_buf: [u8; HEAD_LEN],
    #[allow(unused)]
    body_buf: Box<[u8; BODY_BUF_LENGTH]>,
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub(self) struct HandlerParameters {
    #[allow(unused)]
    pub(self) buffer: Buffer,
    #[allow(unused)]
    pub(self) stream: (SendStream, RecvStream),
    #[allow(unused)]
    pub(self) outer_stream: Receiver<msg::Msg>,
    #[allow(unused)]
    pub(self) connection_map: ConnectionMap,
    #[allow(unused)]
    pub(self) status_map: StatusMap,
    #[allow(unused)]
    pub(self) redis_ops: RedisOps,
}

#[async_trait]
pub(self) trait Handler: Send + Sync + 'static {
    // the shared part is the function, not the data. So the `self` should be immutable.
    async fn handle_function(
        &self,
        msg: &mut msg::Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<msg::Msg>;
}

pub(super) async fn start() -> Result<()> {
    let server = server::Server::new();
    server.run().await?;
    Ok(())
}

#[allow(unused)]
pub(crate) async fn mock() -> Result<()> {
    let client = mock::Client::new().await?;
    client.echo().await?;
    Ok(())
}

#[allow(unused)]
pub(self) fn get_connection_map() -> ConnectionMap {
    CONNECTION_MAP.clone()
}

#[allow(unused)]
pub(self) fn get_status_map() -> StatusMap {
    STATUS_MAP.clone()
}
