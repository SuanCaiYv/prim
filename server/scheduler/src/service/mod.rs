mod handler;
mod server;

use std::sync::Arc;

use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::{
    net::{server::GenericParameter, OuterSender},
    Result, entity::ServerInfo,
};

pub(crate) struct ClientSenderTimeoutReceiverMap(pub(crate) Arc<DashMap<u32, OuterSender>>);
pub(crate) struct ServerInfoMap(pub(crate) Arc<DashMap<u32, ServerInfo>>);

lazy_static! {
    static ref CLIENT_SENDER_TIMEOUT_RECEIVER_MAP: ClientSenderTimeoutReceiverMap =
        ClientSenderTimeoutReceiverMap(Arc::new(DashMap::new()));
    static ref SERVER_INFO_MAP: ServerInfoMap = ServerInfoMap(Arc::new(DashMap::new()));
}

pub(crate) fn get_client_sender_timeout_receiver_map() -> ClientSenderTimeoutReceiverMap {
    ClientSenderTimeoutReceiverMap(CLIENT_SENDER_TIMEOUT_RECEIVER_MAP.0.clone())
}

pub(crate) fn get_server_info_map() -> ServerInfoMap {
    ServerInfoMap(SERVER_INFO_MAP.0.clone())
}

impl GenericParameter for ClientSenderTimeoutReceiverMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for ServerInfoMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await?;
    Ok(())
}
