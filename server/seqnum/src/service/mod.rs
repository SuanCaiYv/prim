use std::sync::{atomic::{AtomicU64, AtomicBool}, Arc};

use dashmap::DashMap;
use lib::{net::GenericParameter, Result};

use lazy_static::lazy_static;

pub(crate) mod handler;
pub(crate) mod server;

pub(crate) struct SeqnumMap(pub(crate) Arc<DashMap<u128, AtomicU64>>);

lazy_static! {
    static ref SEQNUM_MAP: SeqnumMap = SeqnumMap(Arc::new(DashMap::new()));
    pub(crate) static ref STOP_SIGNAL: AtomicBool = AtomicBool::new(false);
}

pub(crate) fn get_seqnum_map() -> SeqnumMap {
    SeqnumMap(SEQNUM_MAP.0.clone())
}

impl GenericParameter for SeqnumMap {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl SeqnumMap {
    pub(crate) fn insert(&self, key: u128, value: AtomicU64) {
        self.0.insert(key, value);
    }

    #[allow(unused)]
    pub(crate) fn remove(&self, key: &u128) {
        self.0.remove(key);
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}
