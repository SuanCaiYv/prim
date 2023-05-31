use std::{net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use lazy_static::lazy_static;

use lib::{net::ReqwestOperatorManager, util::should_connect_to_peer, Result};

use crate::util::my_id;

use self::client::Client;

pub(self) mod client;

static mut CLIENT: Option<Client> = None;

lazy_static! {
    pub(self) static ref NODE_MAP: Arc<DashMap<u32, ReqwestOperatorManager>> =
        Arc::new(DashMap::new());
}

pub(crate) fn get_node_map() -> Arc<DashMap<u32, ReqwestOperatorManager>> {
    NODE_MAP.clone()
}

pub(crate) async fn node_online(address: SocketAddr, node_id: u32, new_peer: bool) -> Result<()> {
    if should_connect_to_peer(my_id(), node_id, new_peer) {
        let operator = unsafe { CLIENT.as_mut().unwrap().new_connection(address).await? };
        NODE_MAP.insert(node_id, operator);
    }
    Ok(())
}

pub(crate) async fn node_offline(node_id: u32) -> Result<()> {
    NODE_MAP.remove(&node_id);
    Ok(())
}

pub(crate) async fn start() -> Result<()> {
    let mut client = Client::new();
    client.build().await?;
    unsafe {
        CLIENT = Some(client);
    }
    Ok(())
}
