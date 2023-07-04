pub(crate) mod handler;
mod server;

use std::sync::Arc;

use dashmap::{mapref::one::Ref, DashMap, DashSet};
use lazy_static::lazy_static;
use lib::{Result, entity::ServerInfo, net::GenericParameter};
use lib_net_tokio::net::server::ReqwestCaller;

/// we choose to split set and integration map to get minimum split operation.
pub(crate) struct ClientCallerMap(pub(crate) Arc<DashMap<u32, ReqwestCaller>>);
pub(crate) struct ServerInfoMap(pub(crate) Arc<DashMap<u32, ServerInfo>>);
pub(crate) struct MessageNodeSet(pub(crate) Arc<DashSet<u32>>);
pub(crate) struct SeqnumNodeSet(pub(crate) Arc<DashSet<u32>>);

lazy_static! {
    static ref CLIENT_CONNECTION_MAP: ClientCallerMap = ClientCallerMap(Arc::new(DashMap::new()));
    static ref SERVER_INFO_MAP: ServerInfoMap = ServerInfoMap(Arc::new(DashMap::new()));
    static ref MESSAGE_NODE_SET: MessageNodeSet = MessageNodeSet(Arc::new(DashSet::new()));
    static ref SEQNUM_NODE_SET: SeqnumNodeSet = SeqnumNodeSet(Arc::new(DashSet::new()));
}

pub(crate) fn get_client_caller_map() -> ClientCallerMap {
    ClientCallerMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_server_info_map() -> ServerInfoMap {
    ServerInfoMap(SERVER_INFO_MAP.0.clone())
}

pub(crate) fn get_message_node_set() -> MessageNodeSet {
    MessageNodeSet(MESSAGE_NODE_SET.0.clone())
}

pub(crate) fn get_seqnum_node_set() -> SeqnumNodeSet {
    SeqnumNodeSet(SEQNUM_NODE_SET.0.clone())
}

impl GenericParameter for ClientCallerMap {
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

impl GenericParameter for MessageNodeSet {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for SeqnumNodeSet {
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

impl ServerInfoMap {
    pub(crate) fn get(&self, key: u32) -> Option<Ref<u32, ServerInfo>> {
        self.0.get(&key)
    }

    pub(crate) fn insert(&self, key: u32, value: ServerInfo) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&self, key: u32) {
        self.0.remove(&key);
    }
}

impl MessageNodeSet {
    #[allow(unused)]
    pub(crate) fn contains(&self, key: u32) -> bool {
        self.0.contains(&key)
    }

    pub(crate) fn insert(&self, key: u32) {
        self.0.insert(key);
    }

    pub(crate) fn remove(&self, key: u32) {
        self.0.remove(&key);
    }
}

impl SeqnumNodeSet {
    #[allow(unused)]
    pub(crate) fn contains(&self, key: u32) -> bool {
        self.0.contains(&key)
    }

    pub(crate) fn insert(&self, key: u32) {
        self.0.insert(key);
    }

    pub(crate) fn remove(&self, key: u32) {
        self.0.remove(&key);
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await?;
    Ok(())
}
