use std::sync::Arc;

use dashmap::DashMap;
use lib::{Result, net::ReqwestOperatorManager};

pub(crate) mod handler;
pub(crate) mod server;

static mut CLIENT_MAP: Option<Arc<DashMap<u32, ReqwestOperatorManager>>> = None;

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}

pub(crate) fn get_client_map() -> Arc<DashMap<u32, ReqwestOperatorManager>> {
    unsafe {
        CLIENT_MAP.as_ref().unwrap().clone()
    }
}
