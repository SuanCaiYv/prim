use super::{InnerSender, LenBuffer, OuterReceiver, OuterSender, ALPN_PRIM};
use crate::{entity::Msg, net::MsgIO, Result};
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use quinn::{NewConnection, VarInt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::select;
use tracing::{debug, info, error};

pub type NewConnectionHandlerGenerator =
Box<dyn Fn() -> Box<dyn NewConnectionHandler> + Send + Sync + 'static>;
pub type IOSender = OuterSender;
pub type IOReceiver = OuterReceiver;

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);

pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;

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
    pub io_handler_sender: InnerSender,
    pub generic_parameters: GenericParameterMap,
}

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// the [`msg`] should be read only, and if you want to change it, use copy-on-write... as saying `clone` it.
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg>;
}

#[async_trait]
pub trait NewConnectionHandler: Send + Sync + 'static {
    async fn handle(&mut self, io_channel: (IOSender, IOReceiver)) -> Result<()>;
}

#[allow(unused)]
pub struct ServerConfig {
    address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: VarInt,
    /// the client and server should be the same value.
    connection_idle_timeout: VarInt,
    max_bi_streams: VarInt,
    max_uni_streams: VarInt,
    max_io_channel_size: usize,
    max_task_channel_size: usize,
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
    pub connection_idle_timeout: Option<VarInt>,
    #[allow(unused)]
    pub max_bi_streams: Option<VarInt>,
    #[allow(unused)]
    pub max_uni_streams: Option<VarInt>,
    pub max_io_channel_size: Option<usize>,
    pub max_task_channel_size: Option<usize>,
}

impl Default for ServerConfigBuilder {
    fn default() -> Self {
        Self {
            address: None,
            cert: None,
            key: None,
            max_connections: None,
            connection_idle_timeout: None,
            max_bi_streams: None,
            max_uni_streams: None,
            max_io_channel_size: None,
            max_task_channel_size: None,
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

    pub fn with_max_io_channel_size(&mut self, max_io_channel_size: usize) -> &mut Self {
        self.max_io_channel_size = Some(max_io_channel_size);
        self
    }

    pub fn with_max_task_channel_size(&mut self, max_task_channel_size: usize) -> &mut Self {
        self.max_task_channel_size = Some(max_task_channel_size);
        self
    }

    pub fn build(self) -> Result<ServerConfig> {
        let address = self.address.ok_or_else(|| anyhow!("address is required"))?;
        let cert = self.cert.ok_or_else(|| anyhow!("cert is required"))?;
        let key = self.key.ok_or_else(|| anyhow!("key is required"))?;
        let max_connections = self
            .max_connections
            .ok_or_else(|| anyhow!("max_connections is required"))?;
        let connection_idle_timeout = self
            .connection_idle_timeout
            .ok_or_else(|| anyhow!("connection_idle_timeout is required"))?;
        let max_bi_streams = self
            .max_bi_streams
            .ok_or_else(|| anyhow!("max_bi_streams is required"))?;
        let max_uni_streams = self
            .max_uni_streams
            .ok_or_else(|| anyhow!("max_uni_streams is required"))?;
        let max_io_channel_size = self.max_io_channel_size.ok_or_else(|| anyhow!("max_io_channel_size is required"))?;
        let max_task_channel_size = self.max_task_channel_size.ok_or_else(|| anyhow!("max_task_channel_size is required"))?;
        Ok(ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
            max_io_channel_size,
            max_task_channel_size,
        })
    }
}

pub struct Server {
    config: Option<ServerConfig>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub async fn run(&mut self, generator: NewConnectionHandlerGenerator) -> Result<()> {
        // deconstruct Server
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
            max_io_channel_size,
            max_task_channel_size,
        } = self.config.take().unwrap();
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
            let handler = generator();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, handler, max_io_channel_size, max_task_channel_size).await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn handle_new_connection(
        mut conn: NewConnection,
        mut handler: Box<dyn NewConnectionHandler>,
        max_io_channel_size: usize,
        max_task_channel_size: usize,
    ) -> Result<()> {
        let (inner_sender, outer_receiver) = tokio::sync::mpsc::channel(max_io_channel_size);
        let (outer_sender, inner_receiver) = async_channel::bounded(max_task_channel_size);
        tokio::spawn(async move {
            let _ = handler.handle((outer_sender, outer_receiver)).await;
        });
        let mut quickly_close = tokio::sync::mpsc::channel(64);
        loop {
            select! {
                streams = conn.bi_streams.next() => {
                    if let Some(streams) = streams {
                        let io_streams = match streams {
                            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                                error!("the peer close the connection.");
                                Err(anyhow!("the peer close the connection."))
                            }
                            Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                                error!("the peer close the connection but by quic.");
                                Err(anyhow!("the peer close the connection but by quic."))
                            }
                            Err(quinn::ConnectionError::Reset) => {
                                error!("connection reset.");
                                Err(anyhow!("connection reset."))
                            }
                            Err(quinn::ConnectionError::TransportError { .. }) => {
                                error!("connect by fake specification.");
                                Err(anyhow!("connect by fake specification."))
                            }
                            Err(quinn::ConnectionError::TimedOut) => {
                                error!("connection idle for too long time.");
                                Err(anyhow!("connection idle for too long time."))
                            }
                            Err(quinn::ConnectionError::VersionMismatch) => {
                                error!("connect by unsupported protocol version.");
                                Err(anyhow!("connect by unsupported protocol version."))
                            }
                            Err(quinn::ConnectionError::LocallyClosed) => {
                                error!("local server fatal.");
                                Err(anyhow!("local server fatal."))
                            }
                            Ok(ok) => Ok(ok),
                        };
                        if let Ok(mut io_streams) = io_streams {
                            let (inner_sender, inner_receiver) = (inner_sender.clone(), inner_receiver.clone());
                            let quickly_close_sender = quickly_close.0.clone();
                            tokio::spawn(async move {
                                let mut buf: Box<LenBuffer> = Box::new([0_u8; 4]);
                                loop {
                                    select! {
                                        msg = MsgIO::read_msg(&mut buf, &mut io_streams.1) => {
                                            match msg {
                                                Ok(msg) => {
                                                    if let Err(_) = inner_sender.send(msg).await {
                                                        break;
                                                    }
                                                },
                                                Err(_) => {
                                                    break;
                                                },
                                            }
                                        },
                                        msg = inner_receiver.recv() => {
                                            match msg {
                                                Ok(msg) => {
                                                    if let Err(_) = MsgIO::write_msg(msg, &mut io_streams.0).await {
                                                        break;
                                                    }
                                                },
                                                Err(_) => {
                                                    let _ = quickly_close_sender.send(()).await;
                                                    break;
                                                },
                                            }
                                        },
                                    }
                                }
                            });
                        } else {
                            break;
                        }
                    }
                },
                _ = quickly_close.1.recv() => {
                    break;
                },
            }
        }
        debug!("connection closed.");
        conn.connection
            .close(0u32.into(), b"it's time to say goodbye.");
        Ok(())
    }
}
