use crate::entity::Msg;
use crate::net::{InnerReceiver, InnerSender, LenBuffer, ALPN_PRIM};
use crate::Result;
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use quinn::{NewConnection, RecvStream, SendStream, VarInt};
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tracing::{info, warn};

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);
pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;
pub type ConnectionTaskGenerator =
    Box<dyn Fn(NewConnection) -> Box<dyn ConnectionTask> + Send + Sync + 'static>;

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Result<&T> {
        let parameter = self.0.get(std::any::type_name::<T>());
        if parameter.is_none() {
            Err(anyhow!("parameter not found"))
        } else {
            let parameter = parameter.unwrap();
            let parameter = parameter.as_any().downcast_ref::<T>();
            if parameter.is_none() {
                Err(anyhow!("parameter type mismatch"))
            } else {
                Ok(parameter.unwrap())
            }
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Result<&mut T> {
        let parameter = self.0.get_mut(std::any::type_name::<T>());
        if parameter.is_none() {
            Err(anyhow!("parameter not found"))
        } else {
            let parameter = parameter.unwrap();
            let parameter = parameter.as_mut_any().downcast_mut::<T>();
            if parameter.is_none() {
                Err(anyhow!("parameter type mismatch"))
            } else {
                Ok(parameter.unwrap())
            }
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub struct HandlerParameters {
    #[allow(unused)]
    pub buffer: LenBuffer,
    /// in/out streams interacting with quic
    #[allow(unused)]
    pub streams: (SendStream, RecvStream),
    /// inner streams interacting with other tasks
    /// why tokio? cause this direction's model is multi-sender and single-receiver
    /// why async-channel? cause this direction's model is single-sender multi-receiver
    pub inner_streams: (InnerSender, InnerReceiver),
    #[allow(unused)]
    pub generic_parameters: GenericParameterMap,
}

#[async_trait]
pub trait ConnectionTask: Send + Sync + 'static {
    /// this method will run in a new tokio task.
    async fn handle(mut self: Box<Self>) -> Result<()>;
}

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// the [`msg`] should be read only, and if you want to change it, use copy-on-write... as saying `clone` it.
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg>;
}

#[allow(unused)]
pub struct ServerConfig {
    address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: VarInt,
    /// should set only on clients.
    keep_alive_interval: Duration,
    /// the client and server should be the same value.
    connection_idle_timeout: VarInt,
    max_bi_streams: VarInt,
    max_uni_streams: VarInt,
}

pub struct ServerConfigBuilder {
    #[allow(unused)]
    pub address: Option<SocketAddr>,
    #[allow(unused)]
    pub cert: Option<rustls::Certificate>,
    #[allow(unused)]
    pub key: Option<rustls::PrivateKey>,
    #[allow(unused)]
    pub max_connections: Option<VarInt>,
    #[allow(unused)]
    pub keep_alive_interval: Option<Duration>,
    #[allow(unused)]
    pub connection_idle_timeout: Option<VarInt>,
    #[allow(unused)]
    pub max_bi_streams: Option<VarInt>,
    #[allow(unused)]
    pub max_uni_streams: Option<VarInt>,
}

impl Default for ServerConfigBuilder {
    fn default() -> Self {
        Self {
            address: None,
            cert: None,
            key: None,
            max_connections: None,
            keep_alive_interval: None,
            connection_idle_timeout: None,
            max_bi_streams: None,
            max_uni_streams: None,
        }
    }
}

impl ServerConfigBuilder {
    pub fn with_address(&mut self, address: SocketAddr) -> &mut Self {
        self.address = Some(address);
        self
    }

    pub fn with_cert(&mut self, cert: rustls::Certificate) -> &mut Self {
        self.cert = Some(cert);
        self
    }

    pub fn with_key(&mut self, key: rustls::PrivateKey) -> &mut Self {
        self.key = Some(key);
        self
    }

    pub fn with_max_connections(&mut self, max_connections: VarInt) -> &mut Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn with_keep_alive_interval(&mut self, keep_alive_interval: Duration) -> &mut Self {
        self.keep_alive_interval = Some(keep_alive_interval);
        self
    }

    pub fn with_connection_idle_timeout(&mut self, connection_idle_timeout: VarInt) -> &mut Self {
        self.connection_idle_timeout = Some(connection_idle_timeout);
        self
    }

    pub fn with_max_bi_streams(&mut self, max_bi_streams: VarInt) -> &mut Self {
        self.max_bi_streams = Some(max_bi_streams);
        self
    }

    pub fn with_max_uni_streams(&mut self, max_uni_streams: VarInt) -> &mut Self {
        self.max_uni_streams = Some(max_uni_streams);
        self
    }

    pub fn build(self) -> Result<ServerConfig> {
        let address = self.address.ok_or_else(|| anyhow!("address is required"))?;
        let cert = self.cert.ok_or_else(|| anyhow!("cert is required"))?;
        let key = self.key.ok_or_else(|| anyhow!("key is required"))?;
        let max_connections = self
            .max_connections
            .ok_or_else(|| anyhow!("max_connections is required"))?;
        let keep_alive_interval = self
            .keep_alive_interval
            .ok_or_else(|| anyhow!("keep_alive_interval is required"))?;
        let connection_idle_timeout = self
            .connection_idle_timeout
            .ok_or_else(|| anyhow!("connection_idle_timeout is required"))?;
        let max_bi_streams = self
            .max_bi_streams
            .ok_or_else(|| anyhow!("max_bi_streams is required"))?;
        let max_uni_streams = self
            .max_uni_streams
            .ok_or_else(|| anyhow!("max_uni_streams is required"))?;
        Ok(ServerConfig {
            address,
            cert,
            key,
            max_connections,
            keep_alive_interval,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
        })
    }
}

/// the server is multi-connection designed.
/// That means the minimum unit to handle is [`quinn::NewConnection`]
pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    /// why don't we wrapping operations on I/O and just return two channel to send and receive [`crate::entity::Msg`]?
    /// the answer is `dealing msg on server side can be more complex than client side`.
    /// such as the `auth` msg will drop the connection when authentication failed and it always the first msg sent.
    /// so we need to handle the first stream with it's first read specially.
    pub async fn run(self, connection_task_generator: ConnectionTaskGenerator) -> Result<()> {
        // deconstruct Server
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            keep_alive_interval: _,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
        } = self.config;
        // set crypto for server
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        // set custom alpn protocol
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        // set max concurrent connections
        quinn_server_config.concurrent_connections(max_connections.into_inner() as u32);
        quinn_server_config.use_retry(true);
        // set quic transport parameters
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(max_bi_streams)
            .max_concurrent_uni_streams(max_uni_streams)
            // the keep-alive interval should set on client.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(connection_idle_timeout)));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let handler = connection_task_generator(conn);
            tokio::spawn(async move {
                let _ = handler.handle().await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}

pub struct ConnectionUtil;

impl ConnectionUtil {
    /// first stream failed will cause closed of the connection.
    pub async fn first_stream(conn: &mut NewConnection) -> Result<(SendStream, RecvStream)> {
        if let Some(streams) = conn.bi_streams.next().await {
            if let Ok(streams) = streams {
                Ok(streams)
            } else {
                conn.connection
                    .close(VarInt::from(1_u8), b"first stream failed.");
                return Err(anyhow!("first stream fatal."));
            }
        } else {
            conn.connection
                .close(VarInt::from(1_u8), "first stream open failed.".as_bytes());
            return Err(anyhow!("first stream open fatal."));
        }
    }

    /// when open streams failed, connection will not be closed, this should be handled by caller with their own logic.
    pub async fn more_stream(conn: &mut NewConnection) -> Result<(SendStream, RecvStream)> {
        let streams = conn.bi_streams.next().await;
        if streams.is_none() {
            warn!("connection closed.");
            return Err(anyhow!("connection closed."));
        }
        let streams = streams.unwrap();
        match streams {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                info!("the peer close the connection.");
                Err(anyhow!("the peer close the connection."))
            }
            Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                info!("the peer close the connection but by quic.");
                Err(anyhow!("the peer close the connection but by quic."))
            }
            Err(quinn::ConnectionError::Reset) => {
                info!("connection reset.");
                Err(anyhow!("connection reset."))
            }
            Err(quinn::ConnectionError::TransportError { .. }) => {
                warn!("connect by fake specification.");
                Err(anyhow!("connect by fake specification."))
            }
            Err(quinn::ConnectionError::TimedOut) => {
                warn!("connection idle for too long time.");
                Err(anyhow!("connection idle for too long time."))
            }
            Err(quinn::ConnectionError::VersionMismatch) => {
                warn!("connect by unsupported protocol version.");
                Err(anyhow!("connect by unsupported protocol version."))
            }
            Err(quinn::ConnectionError::LocallyClosed) => {
                warn!("local server fatal.");
                Err(anyhow!("local server fatal."))
            }
            Ok(ok) => Ok(ok),
        }
    }
}
