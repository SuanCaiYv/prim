use std::sync::{atomic::AtomicU64, Arc};

use dashmap::{mapref::one::Ref, DashMap};
use lib::{net::GenericParameter, Result};

use lazy_static::lazy_static;
use lib_net_tokio::net::server::ReqwestCaller;

pub(crate) mod handler;
pub(crate) mod server;

static mut CLIENT_MAP: Option<ClientCallerMap> = None;

pub(crate) struct ClientCallerMap(pub(crate) Arc<DashMap<u32, ReqwestCaller>>);
pub(crate) struct SeqnumMap(pub(crate) Arc<DashMap<u128, Arc<AtomicU64>>>);

lazy_static! {
    static ref CLIENT_CONNECTION_MAP: ClientCallerMap = ClientCallerMap(Arc::new(DashMap::new()));
    static ref SEQNUM_MAP: SeqnumMap = SeqnumMap(Arc::new(DashMap::new()));
}

pub(crate) fn get_client_caller_map() -> ClientCallerMap {
    ClientCallerMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_seqnum_map() -> SeqnumMap {
    SeqnumMap(SEQNUM_MAP.0.clone())
}

impl GenericParameter for ClientCallerMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for SeqnumMap {
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

impl SeqnumMap {
    pub(crate) fn get(&self, key: &u128) -> Option<Ref<'_, u128, Arc<AtomicU64>>> {
        self.0.get(key)
    }

    pub(crate) fn insert(&self, key: u128, value: Arc<AtomicU64>) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&self, key: &u128) {
        self.0.remove(key);
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}
