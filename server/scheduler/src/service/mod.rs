pub(crate) mod handler;
mod server;

use std::sync::Arc;

use dashmap::{mapref::one::Ref, DashMap, DashSet};
use lazy_static::lazy_static;
use lib::{
    entity::ServerInfo,
    net::{server::GenericParameter, MsgSender},
    Result,
};

/// we choose to split set and integration map to get minimum split operation.
pub(crate) struct ClientConnectionMap(pub(crate) Arc<DashMap<u32, MsgSender>>);
pub(crate) struct ServerInfoMap(pub(crate) Arc<DashMap<u32, ServerInfo>>);
pub(crate) struct MessageNodeSet(pub(crate) Arc<DashSet<u32>>);
pub(crate) struct SchedulerNodeSet(pub(crate) Arc<DashSet<u32>>);

lazy_static! {
    static ref CLIENT_CONNECTION_MAP: ClientConnectionMap =
        ClientConnectionMap(Arc::new(DashMap::new()));
    static ref SERVER_INFO_MAP: ServerInfoMap = ServerInfoMap(Arc::new(DashMap::new()));
    static ref MESSAGE_NODE_SET: MessageNodeSet = MessageNodeSet(Arc::new(DashSet::new()));
    static ref SCHEDULER_NODE_SET: SchedulerNodeSet = SchedulerNodeSet(Arc::new(DashSet::new()));
}

pub(crate) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CLIENT_CONNECTION_MAP.0.clone())
}

pub(crate) fn get_server_info_map() -> ServerInfoMap {
    ServerInfoMap(SERVER_INFO_MAP.0.clone())
}

pub(crate) fn get_message_node_set() -> MessageNodeSet {
    MessageNodeSet(MESSAGE_NODE_SET.0.clone())
}

pub(crate) fn get_scheduler_node_set() -> SchedulerNodeSet {
    SchedulerNodeSet(SCHEDULER_NODE_SET.0.clone())
}

impl GenericParameter for ClientConnectionMap {
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

impl GenericParameter for SchedulerNodeSet {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ClientConnectionMap {
    pub(crate) fn get(&self, key: &u32) -> Option<Ref<'_, u32, MsgSender>> {
        self.0.get(key)
    }

    pub(crate) fn insert(&self, key: u32, value: MsgSender) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&self, key: &u32) {
        self.0.remove(key);
    }
}

impl ServerInfoMap {
    pub(crate) fn get(&self, key: &u32) -> Option<ServerInfo> {
        self.0.get(key).map(|v| v.value().clone())
    }

    pub(crate) fn insert(&self, key: u32, value: ServerInfo) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&self, key: &u32) {
        self.0.remove(key);
    }
}

impl MessageNodeSet {
    #[allow(unused)]
    pub(crate) fn contains(&self, key: &u32) -> bool {
        self.0.contains(key)
    }

    pub(crate) fn insert(&self, key: u32) {
        self.0.insert(key);
    }

    pub(crate) fn remove(&self, key: &u32) {
        self.0.remove(key);
    }
}

impl SchedulerNodeSet {
    #[allow(unused)]
    pub(crate) fn contains(&self, key: &u32) -> bool {
        self.0.contains(key)
    }

    pub(crate) fn insert(&self, key: u32) {
        self.0.insert(key);
    }

    pub(crate) fn remove(&self, key: &u32) {
        self.0.remove(key);
    }
}

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await?;
    Ok(())
}
