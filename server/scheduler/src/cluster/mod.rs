mod client;
mod handler;
mod server;

use std::{net::SocketAddr, sync::Arc};

use dashmap::{mapref::one::Ref, DashMap, DashSet};
use lazy_static::lazy_static;
use lib::{
    net::{server::GenericParameter, MsgSender},
    Result,
};
use tracing::error;

pub(crate) struct ClusterConnectionMap(pub(crate) Arc<DashMap<u32, MsgSender>>);
pub(self) struct ClusterConnectionSet(Arc<DashSet<SocketAddr>>);

lazy_static! {
    static ref CLUSTER_CONNECTION_MAP: ClusterConnectionMap =
        ClusterConnectionMap(Arc::new(DashMap::new()));
    static ref CLUSTER_CONNECTION_SET: ClusterConnectionSet =
        ClusterConnectionSet(Arc::new(DashSet::new()));
}

pub(crate) fn get_cluster_connection_map() -> ClusterConnectionMap {
    ClusterConnectionMap(CLUSTER_CONNECTION_MAP.0.clone())
}

pub(self) fn get_cluster_connection_set() -> ClusterConnectionSet {
    ClusterConnectionSet(CLUSTER_CONNECTION_SET.0.clone())
}

impl GenericParameter for ClusterConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for ClusterConnectionSet {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClusterConnectionMap {
    pub(crate) fn insert(&self, id: u32, sender: MsgSender) {
        self.0.insert(id, sender);
    }

    #[allow(unused)]
    pub(crate) fn remove(&self, id: u32) {
        self.0.remove(&id);
    }

    #[allow(unused)]
    pub(crate) fn get(&self, id: u32) -> Option<Ref<'_, u32, MsgSender>> {
        self.0.get(&id)
    }
}

impl ClusterConnectionSet {
    #[allow(unused)]
    pub(crate) fn insert(&self, addr: SocketAddr) {
        self.0.insert(addr);
    }

    #[allow(unused)]
    pub(crate) fn remove(&self, addr: &SocketAddr) {
        self.0.remove(addr);
    }

    #[allow(unused)]
    pub(crate) fn contains(&self, addr: &SocketAddr) -> bool {
        self.0.contains(addr)
    }
}

pub(crate) async fn start() -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = client::Client::run().await {
            error!("cluster client error: {}", e);
        }
    });
    server::Server::run().await?;
    Ok(())
}
