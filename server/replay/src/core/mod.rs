use std::{net::SocketAddr, sync::Arc};

use common::net::InnerSender;
use common::Result;
use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;
mod inner;
pub mod outer;

pub(self) type ClusterConnectionSet = Arc<DashMap<SocketAddr, InnerSender>>;
pub(self) type AckMap = Arc<DashSet<String>>;

lazy_static! {
    pub(self) static ref CLUSTER_CONNECTION_SET: ClusterConnectionSet =
        ClusterConnectionSet::new(DashMap::new());
    pub(self) static ref ACK_MAP: AckMap = Arc::new(DashSet::new());
}

pub(crate) fn get_ack_map() -> AckMap {
    ACK_MAP.clone()
}

pub(crate) fn get_cluster_connection_set() -> ClusterConnectionSet {
    CLUSTER_CONNECTION_SET.clone()
}

pub(crate) async fn start() -> Result<()> {
    inner::start().await?;
    outer::start().await?;
    Ok(())
}
