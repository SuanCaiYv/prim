use async_channel::{Receiver, Sender};
use quinn::{RecvStream, SendStream};
use std::sync::Arc;

use dashmap::DashMap;
use structopt::lazy_static::lazy_static;

use tonic::async_trait;

use crate::cache::redis_ops::RedisOps;
use crate::entity::Msg;
pub(self) mod handler;
mod mock;
pub(self) mod server;

pub(self) const BODY_SIZE: usize = 1 << 16;
pub(self) const ALPN_PRIM: &[&[u8]] = &[b"prim"];

pub(crate) type Result<T> = anyhow::Result<T>;

/// use Arc + ConcurrentMap + Clone to share state between Tasks
pub(self) type ConnectionMap = Arc<DashMap<u64, Sender<Msg>>>;
pub(self) type StatusMap = Arc<DashMap<u64, u64>>;
pub(self) type LenBuffer = [u8; 4];

lazy_static! {
    static ref CONNECTION_MAP: ConnectionMap = Arc::new(DashMap::new());
    static ref STATUS_MAP: StatusMap = Arc::new(DashMap::new());
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub(self) struct HandlerParameters {
    #[allow(unused)]
    pub(self) buffer: LenBuffer,
    #[allow(unused)]
    pub(self) stream: (SendStream, RecvStream),
    #[allow(unused)]
    pub(self) outer_stream: Receiver<Msg>,
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
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> Result<Msg>;
}

pub(super) async fn start() -> Result<()> {
    let server = server::Server::new();
    server.run().await?;
    Ok(())
}

#[allow(unused)]
pub(crate) async fn mock() -> Result<()> {
    let client = mock::Client::new(None).await?;
    client.echo().await?;
    Ok(())
}

pub(crate) async fn mock_peer() -> Result<()> {
    let c1 = mock::Client::new(Some("[::1]:8190".to_string())).await?;
    let c2 = mock::Client::new(Some("[::1]:8290".to_string())).await?;
    mock::Client::echo_you_and_me(c1, c2, 115, 916).await?;
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
