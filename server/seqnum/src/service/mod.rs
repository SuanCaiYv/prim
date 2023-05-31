use std::sync::Arc;

use dashmap::{DashMap, mapref::one::Ref};
use lib::{Result, net::server::{ReqwestCaller, GenericParameter}};

use lazy_static::lazy_static;

pub(crate) mod handler;
pub(crate) mod server;

static mut CLIENT_MAP: Option<ClientCallerMap> = None;

pub(crate) struct ClientCallerMap(pub(crate) Arc<DashMap<u32, ReqwestCaller>>);

lazy_static! {
    static ref CLIENT_CONNECTION_MAP: ClientCallerMap = ClientCallerMap(Arc::new(DashMap::new()));
}

pub(crate) fn get_client_caller_map() -> ClientCallerMap {
    ClientCallerMap(CLIENT_CONNECTION_MAP.0.clone())
}

impl GenericParameter for ClientCallerMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClientCallerMap {
    pub(crate) fn get(&self, key: u32) -> Option<Ref<'_, u32, ReqwestCaller>> {
        self.0.get(&key)
    }

    pub(crate) fn insert(&self, key: u32, value: ReqwestCaller) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&self, key: u32) {
        self.0.remove(&key);
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}
