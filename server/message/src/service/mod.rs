use std::sync::Arc;

use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::{
    net::{server::GenericParameter, OuterSender},
    Result,
};

pub(self) mod handler;
pub(self) mod server;

pub(self) struct ClientConnectionMap(pub(self) Arc<DashMap<u64, OuterSender>>);

lazy_static! {
    pub(self) static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
}

pub(self) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
