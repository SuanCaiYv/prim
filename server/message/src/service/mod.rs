use std::sync::Arc;

use crate::service::handler::{IOTaskReceiver};
use dashmap::{DashMap, mapref::one::Ref};
use lazy_static::lazy_static;
use lib::{
    net::{server::GenericParameter, MsgSender},
    Result,
};
use tracing::error;

use self::handler::io_task;

pub(crate) mod handler;
pub(crate) mod server;

#[derive(Clone)]
pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u64, MsgSender>>);

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

impl ClientConnectionMap {
    pub(crate) fn get<'a>(&'a self, id: &u64) -> Option<Ref<'a, u64, MsgSender>> {
        self.0.get(id)
    }

    pub(crate) fn insert(&self, id: u64, sender: MsgSender) {
        self.0.insert(id, sender);
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
