use std::sync::Arc;

use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::{
    net::{server::GenericParameter, InnerSender, OuterReceiver, OuterSender},
    Result,
};
use tracing::error;

use self::handler::io_task;

pub(crate) mod handler;
pub(self) mod server;

pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, OuterSender>>);

lazy_static! {
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
}

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub(crate) async fn start(io_task_channel: (InnerSender, OuterReceiver)) -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = io_task(io_task_channel.1).await {
            error!("io task error: {}", e);
        }
    });
    server::Server::run(io_task_channel.0).await?;
    Ok(())
}
