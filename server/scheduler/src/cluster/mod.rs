mod client;
mod handler;
mod server;

use std::{net::SocketAddr, sync::Arc};

use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;
use lib::{net::OuterSender, Result};
use tracing::error;

pub(self) struct ClusterSenderTimeoutReceiverMap(Arc<DashMap<u32, OuterSender>>);
pub(self) type ClusterConnectionSet = Arc<DashSet<SocketAddr>>;

lazy_static! {
    static ref CLUSTER_SENDER_TIMEOUT_RECEIVER_MAP: ClusterSenderTimeoutReceiverMap =
        ClusterSenderTimeoutReceiverMap(Arc::new(DashMap::new()));
    static ref CLUSTER_CONNECTION_SET: ClusterConnectionSet = Arc::new(DashSet::new());
}

pub(self) fn get_cluster_sender_timeout_receiver_map() -> ClusterSenderTimeoutReceiverMap {
    ClusterSenderTimeoutReceiverMap(CLUSTER_SENDER_TIMEOUT_RECEIVER_MAP.0.clone())
}

pub(self) fn get_cluster_connection_set() -> ClusterConnectionSet {
    CLUSTER_CONNECTION_SET.clone()
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
