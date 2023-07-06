mod client;
mod handler;
mod server;

use std::{net::SocketAddr, sync::Arc};

use dashmap::{mapref::one::Ref, DashMap};
use lazy_static::lazy_static;
use lib::{
    entity::Msg,
    util::should_connect_to_peer,
    Result, net::GenericParameter,
};
use lib_net_tokio::net::MsgSender;

use crate::{cluster::client::Client, util::my_id};

pub(crate) struct ClusterConnectionMap(pub(crate) Arc<DashMap<u32, MsgSender>>);

lazy_static! {
    static ref CLUSTER_CONNECTION_MAP: ClusterConnectionMap =
        ClusterConnectionMap(Arc::new(DashMap::new()));
    static ref CLUSTER_CLIENT: Client = Client::new();
}

impl GenericParameter for ClusterConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClusterConnectionMap {
    pub(crate) fn get<'a>(&'a self, id: &u32) -> Option<Ref<'a, u32, MsgSender>> {
        self.0.get(id)
    }

    pub(crate) fn insert(&self, id: u32, sender: MsgSender) {
        self.0.insert(id, sender);
    }
}

pub(crate) fn get_cluster_connection_map() -> ClusterConnectionMap {
    ClusterConnectionMap(CLUSTER_CONNECTION_MAP.0.clone())
}

pub(crate) async fn node_online(address: SocketAddr, node_id: u32, new_peer: bool) -> Result<()> {
    if should_connect_to_peer(my_id(), node_id, new_peer) {
        CLUSTER_CLIENT.new_connection(address).await?;
    }
    Ok(())
}

pub(crate) async fn node_offline(node_id: u32) -> Result<()> {
    CLUSTER_CONNECTION_MAP.0.remove(&node_id);
    Ok(())
}

#[allow(unused)]
pub(crate) async fn node_crash(msg: Arc<Msg>) -> Result<()> {
    todo!("node_crash");
}

#[allow(unused)]
pub(crate) async fn start() -> Result<()> {
    server::Server::run().await?;
    Ok(())
}
