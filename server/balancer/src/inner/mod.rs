use quinn::NewConnection;
use redis::{FromRedisValue, RedisError, RedisResult, ToRedisArgs, Value};
use std::any::Any;
use std::net::SocketAddr;

use std::sync::Arc;

use dashmap::DashMap;
use lazy_static::lazy_static;
use tracing::error;

use crate::config::CONFIG;
use crate::inner::handler::auth::Auth;
use crate::inner::server::BalancerConnectionTask;
use common::net::server::{
    ConnectionTaskGenerator, GenericParameter, HandlerList, Server, ServerConfigBuilder,
};
use common::net::OuterSender;
use common::net::{InnerSender, OuterReceiver};
use common::Result;
use handler::register::Register;

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct NodeInfo {
    /// same as sender_id
    pub(crate) id: u64,
    pub(crate) addr: SocketAddr,
    pub(crate) connection_id: u64,
    pub(crate) status: u64,
}

impl ToRedisArgs for NodeInfo {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let res = serde_json::to_vec(self);
        if res.is_err() {
            error!("failed to serialize NodeInfo to json");
        } else {
            let json = res.unwrap();
            json.write_redis_args(out);
        }
    }
}

impl FromRedisValue for NodeInfo {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        if let Value::Data(data) = v {
            let res: serde_json::error::Result<NodeInfo> = serde_json::from_slice(data.as_slice());
            if res.is_err() {
                error!("failed to deserialize NodeInfo from json");
                return Err(RedisError::from((
                    redis::ErrorKind::TypeError,
                    "failed to deserialize NodeInfo from json",
                )));
            } else {
                Ok(res.unwrap())
            }
        } else {
            error!("redis read value type unmatched");
            return Err(RedisError::from((
                redis::ErrorKind::TypeError,
                "redis read value type unmatched",
            )));
        }
    }
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
                global_sender: global_channel.0.clone(),
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
