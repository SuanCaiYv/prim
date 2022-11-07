use self::cluster::{ClientToBalancer, ClusterClient};
use self::handler::business::{Group, Relationship};
use self::handler::io_tasks;
use self::handler::logic::{Auth, Echo};
use self::handler::message::Text;
use crate::core::mock::echo;
use crate::core::server::MessageConnectionHandler;
use crate::CONFIG;
use ahash::AHashSet;
use common::net::client::ClientSubConnection;
use common::net::server::{
    GenericParameter, HandlerList, NewConnectionHandlerGenerator, Server, ServerConfigBuilder,
};
use common::net::{InnerSender, OuterReceiver, OuterSender};
use common::util::is_ipv6_enabled;
use common::Result;
use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::Arc;
use tracing::error;

pub(crate) mod cluster;
pub(self) mod handler;
mod mock;
pub(self) mod server;

/// use Arc + ConcurrentMap + Clone to share state between Tasks
pub(self) struct ConnectionMap(Arc<DashMap<u64, OuterSender>>);
pub(self) struct StatusMap(Arc<DashMap<u64, u64>>);
pub(self) struct GroupUserList(Arc<DashMap<u64, AHashSet<u64>>>);
pub(self) struct GroupRecordedUserId(Arc<DashSet<u64>>);
/// map of node_id and node connection
pub(crate) type ClusterClientMap =
    Arc<DashMap<u32, (OuterSender, OuterReceiver, ClientSubConnection)>>;
pub(self) type ClusterSender = InnerSender;
pub(self) type ClusterReceiver = OuterReceiver;

lazy_static! {
    static ref CONNECTION_MAP: ConnectionMap = ConnectionMap(Arc::new(DashMap::new()));
    static ref STATUS_MAP: StatusMap = StatusMap(Arc::new(DashMap::new()));
    static ref GROUP_USER_LIST: GroupUserList = GroupUserList(Arc::new(DashMap::new()));
    static ref GROUP_RECORDED_USER_ID: GroupRecordedUserId =
        GroupRecordedUserId(Arc::new(DashSet::new()));
    static ref CLUSTER_CLIENT_MAP: ClusterClientMap = Arc::new(DashMap::new());
}

impl GenericParameter for ConnectionMap {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl GenericParameter for StatusMap {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub(super) async fn start() -> Result<()> {
    let outer_channel: (InnerSender, OuterReceiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_task_channel_size);
    let mut handler_list: HandlerList = Arc::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Auth {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Echo {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Text {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Relationship {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Group {}));
    let new_connection_handler_generator: NewConnectionHandlerGenerator = Box::new(move || {
        Box::new(MessageConnectionHandler::new(
            handler_list.clone(),
            outer_channel.0.clone(),
        ))
    });
    let address = CONFIG.server.address;
    if address.is_ipv6() && !is_ipv6_enabled() {
        panic!("ipv6 is not enabled on this machine");
    }
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder
        .with_address(CONFIG.server.address)
        .with_cert(CONFIG.server.cert.clone())
        .with_key(CONFIG.server.key.clone())
        .with_max_connections(CONFIG.server.max_connections)
        .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams)
        .with_max_task_channel_size(CONFIG.performance.max_task_channel_size)
        .with_max_io_channel_size(CONFIG.performance.max_io_channel_size);
    let server_config = server_config_builder.build();
    let mut server = Server::new(server_config.unwrap());
    let cluster_channel = tokio::sync::mpsc::channel(512);
    tokio::spawn(async move {
        let res = io_tasks(outer_channel.1).await;
        if let Err(e) = res {
            error!("io_tasks error: {}", e);
        }
    });
    tokio::spawn(async move {
        let res = ClientToBalancer::new(cluster_channel.0)
            .registry_self()
            .await;
        if let Err(e) = res {
            error!("client to balancer error: {}", e);
        }
    });
    tokio::spawn(async move {
        let res = ClusterClient::new(cluster_channel.1)
            .await
            .unwrap()
            .run()
            .await;
        if let Err(e) = res {
            error!("cluster client error: {}", e);
        }
    });
    server.run(new_connection_handler_generator).await?;
    Ok(())
}

#[allow(unused)]
pub(crate) async fn mock() -> Result<()> {
    echo(115, 916).await?;
    Ok(())
}

#[allow(unused)]
pub(self) fn get_connection_map() -> ConnectionMap {
    ConnectionMap(CONNECTION_MAP.0.clone())
}

#[allow(unused)]
pub(self) fn get_status_map() -> StatusMap {
    StatusMap(STATUS_MAP.0.clone())
}

#[allow(unused)]
pub(self) fn get_group_user_list() -> GroupUserList {
    GroupUserList(GROUP_USER_LIST.0.clone())
}

#[allow(unused)]
pub(self) fn get_group_recorded_user_id() -> GroupRecordedUserId {
    GroupRecordedUserId(GROUP_RECORDED_USER_ID.0.clone())
}

#[allow(unused)]
pub(crate) fn get_cluster_client_map() -> ClusterClientMap {
    CLUSTER_CLIENT_MAP.clone()
}
