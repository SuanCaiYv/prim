use self::cluster::{ClientToBalancer, ClusterClient};
use self::handler::io_tasks;
use self::handler::logic::{Auth, Echo};
use crate::core::mock::echo;
use crate::core::server::MessageConnectionTask;
use crate::CONFIG;
use common::net::client::Client;
use common::net::server::{
    ConnectionTaskGenerator, GenericParameter, HandlerList, Server, ServerConfigBuilder,
};
use common::net::{InnerSender, OuterReceiver, OuterSender};
use common::Result;
use dashmap::DashMap;
use lazy_static::lazy_static;
use quinn::NewConnection;
use std::any::Any;
use std::sync::Arc;

pub(crate) mod cluster;
pub(self) mod handler;
mod mock;
pub(self) mod server;

/// use Arc + ConcurrentMap + Clone to share state between Tasks
pub(self) struct ConnectionMap(Arc<DashMap<u64, OuterSender>>);
pub(self) struct StatusMap(Arc<DashMap<u64, u64>>);
/// map of node_id and node connection
pub(crate) type ClusterClientMap = Arc<DashMap<u64, (OuterSender, OuterReceiver, Client)>>;
pub(self) type ClusterSender = InnerSender;
pub(self) type ClusterReceiver = OuterReceiver;

lazy_static! {
    static ref CONNECTION_MAP: ConnectionMap = ConnectionMap(Arc::new(DashMap::new()));
    static ref STATUS_MAP: StatusMap = StatusMap(Arc::new(DashMap::new()));
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
    let global_channel: (InnerSender, OuterReceiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_inner_connection_channel_buffer_size);
    let mut handler_list: HandlerList = Arc::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Auth {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Echo {}));
    let connection_task_generator: ConnectionTaskGenerator =
        Box::new(move |conn: NewConnection| {
            Box::new(MessageConnectionTask {
                connection: conn,
                handler_list: handler_list.clone(),
                inner_sender: global_channel.0.clone(),
            })
        });
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder
        .with_address(CONFIG.server.address)
        .with_cert(CONFIG.server.cert.clone())
        .with_key(CONFIG.server.key.clone())
        .with_max_connections(CONFIG.server.max_connections)
        .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
        .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams);
    let server_config = server_config_builder.build();
    let server = Server::new(server_config.unwrap());
    let cluster_channel = tokio::sync::mpsc::channel(512);
    tokio::spawn(io_tasks(global_channel.1));
    tokio::spawn(async move {
        let _ = ClientToBalancer::new(cluster_channel.0)
            .registry_self()
            .await;
    });
    tokio::spawn(async move {
        let _ = ClusterClient::new(cluster_channel.1).run().await;
    });
    server.run(connection_task_generator).await?;
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
pub(crate) fn get_cluster_client_map() -> ClusterClientMap {
    CLUSTER_CLIENT_MAP.clone()
}
