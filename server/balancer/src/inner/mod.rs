use crate::config::CONFIG;
use crate::inner::handler::logic::Auth;
use crate::inner::handler::monitor;
use crate::inner::server::BalancerConnectionTask;
use common::entity::NodeInfo;
use common::net::server::{
    ConnectionTaskGenerator, GenericParameter, HandlerList, Server, ServerConfigBuilder,
};
use common::net::OuterSender;
use common::net::{InnerSender, OuterReceiver};
use common::Result;
use dashmap::DashMap;
use handler::internal::Register;
use lazy_static::lazy_static;
use quinn::NewConnection;
use std::any::Any;
use std::sync::Arc;

mod handler;
pub(self) mod server;

/// the map of sender_id and send channel
pub(self) struct ConnectionMap(Arc<DashMap<u64, OuterSender>>);

/// the map of connection_id and server node information
pub(self) struct StatusMap(Arc<DashMap<u64, NodeInfo>>);

/// stable connection id
pub(super) struct ConnectionId(u64);

lazy_static! {
    static ref CONNECTION_MAP: ConnectionMap = ConnectionMap(Arc::new(DashMap::new()));
    static ref STATUS_MAP: StatusMap = StatusMap(Arc::new(DashMap::new()));
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

impl GenericParameter for ConnectionId {
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
        .push(Box::new(Register {}));
    let connection_task_generator: ConnectionTaskGenerator =
        Box::new(move |conn: NewConnection| {
            Box::new(BalancerConnectionTask {
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
    tokio::spawn(monitor(global_channel.1));
    server.run(connection_task_generator).await?;
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
