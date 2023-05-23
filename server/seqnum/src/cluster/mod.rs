use std::{net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use lazy_static::lazy_static;

use lib::{Result, net::ReqwestOperatorManager};

use self::client::Client;

pub(self) mod client;

static mut CLIENT: Option<Client> = None;

lazy_static! {
    pub(self) static ref NODE_MAP: Arc<DashMap<u32, ReqwestOperatorManager>> = Arc::new(DashMap::new());
}

pub(crate) fn node_map() -> Arc<DashMap<u32, ReqwestOperatorManager>> {
    NODE_MAP.clone()
}

pub(crate) async fn node_online(address: SocketAddr, node_id: u32) -> Result<()> {
    let operator = unsafe {
        CLIENT.as_mut().unwrap().new_connection(address).await?
    };
    NODE_MAP.insert(node_id, operator);
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