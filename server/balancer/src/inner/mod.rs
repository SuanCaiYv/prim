use crate::config::CONFIG;
use crate::inner::handler::logic::Auth;
use crate::inner::handler::monitor;
use crate::inner::server::BalancerConnectionHandler;
use common::entity::NodeInfo;
use common::net::server::{
    GenericParameter, HandlerList, NewConnectionHandlerGenerator, Server, ServerConfigBuilder,
};
use common::net::OuterSender;
use common::net::{InnerSender, OuterReceiver};
use common::Result;
use dashmap::DashMap;
use handler::internal::Register;
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::Arc;

mod cluster;
mod handler;
pub(self) mod server;

/// the map of sender_id and send channel
pub(self) struct NodeClientMap(Arc<DashMap<u32, OuterSender>>);
/// the map of sender_id and server node information
pub(crate) struct StatusMap(pub(crate) Arc<DashMap<u32, NodeInfo>>);
/// stable connection id
pub(super) struct ConnectionId(u64);

lazy_static! {
    static ref CONNECTION_MAP: NodeClientMap = NodeClientMap(Arc::new(DashMap::new()));
    static ref STATUS_MAP: StatusMap = StatusMap(Arc::new(DashMap::new()));
}

impl GenericParameter for NodeClientMap {
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

impl GenericParameter for ConnectionId {
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
        .push(Box::new(Register {}));
    let connection_task_generator: NewConnectionHandlerGenerator = Box::new(move || {
        Box::new(BalancerConnectionHandler::new(
            handler_list.clone(),
            outer_channel.0.clone(),
        ))
    });
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder
        .with_address(CONFIG.server.address)
        .with_cert(CONFIG.server.cert.clone())
        .with_key(CONFIG.server.key.clone())
        .with_max_connections(CONFIG.server.max_connections)
        .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams)
        .with_max_io_channel_size(CONFIG.performance.max_io_channel_size)
        .with_max_task_channel_size(CONFIG.performance.max_task_channel_size);
    let server_config = server_config_builder.build();
    let mut server = Server::new(server_config.unwrap());
    server.run(connection_task_generator).await?;
    tokio::spawn(monitor(outer_channel.1));
    Ok(())
}

#[allow(unused)]
pub(self) fn get_node_client_map() -> NodeClientMap {
    NodeClientMap(CONNECTION_MAP.0.clone())
}

#[allow(unused)]
pub(crate) fn get_status_map() -> StatusMap {
    StatusMap(STATUS_MAP.0.clone())
}
