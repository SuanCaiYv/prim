use lib::{net::server::ClientCallerMap, Result};

pub(crate) mod handler;
pub(crate) mod server;

static mut CLIENT_MAP: Option<ClientCallerMap> = None;

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}

pub(crate) fn get_client_map() -> ClientCallerMap {
    unsafe { CLIENT_MAP.as_ref().unwrap().clone() }
}
