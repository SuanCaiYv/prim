mod client;
mod handler;
mod server;

use std::sync::Arc;

use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::{entity::{Msg, ServerInfo}, net::OuterSender, Result};

use crate::util::should_connect_to_peer;

use self::client::Client;

pub(self) struct ClusterSenderTimeoutReceiverMap(Arc<DashMap<u32, OuterSender>>);

lazy_static! {
    static ref CLUSTER_SENDER_TIMEOUT_RECEIVER_MAP: ClusterSenderTimeoutReceiverMap =
        ClusterSenderTimeoutReceiverMap(Arc::new(DashMap::new()));
    static ref CLUSTER_CLIENT: Client = Client::new();
}

pub(self) fn get_cluster_sender_timeout_receiver_map() -> ClusterSenderTimeoutReceiverMap {
    ClusterSenderTimeoutReceiverMap(CLUSTER_SENDER_TIMEOUT_RECEIVER_MAP.0.clone())
}

pub(crate) async fn node_online(msg: Arc<Msg>) -> Result<()> {
    let server_info = ServerInfo::from(msg.payload());
    let new_peer = bool::from(msg.extension());
    if should_connect_to_peer(server_info.id, new_peer) {
        CLUSTER_CLIENT.new_connection(server_info.address).await?;
    }
    Ok(())
}

pub(crate) async fn node_offline(msg: Arc<Msg>) -> Result<()> {
    let server_info = ServerInfo::from(msg.payload());
    CLUSTER_SENDER_TIMEOUT_RECEIVER_MAP.0.remove(&server_info.id);
    Ok(())
}

pub(crate) async fn node_crash(msg: Arc<Msg>) -> Result<()> {
    todo!("node_crash");
}

pub(crate) async fn start() -> Result<()> {
    todo!()
}
