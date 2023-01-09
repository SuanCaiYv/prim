use std::{any::type_name, net::SocketAddr, sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};

use crate::{
    entity::{Msg, Type, HEAD_LEN},
    net::{MsgIOTimeoutUtil, MsgIOUtil},
    Result,
};
use ahash::{AHashMap, AHashSet};
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use quinn::NewConnection;
use tokio::{io::split, net::TcpStream, select};
use tokio::io::AsyncWriteExt;
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info};

use super::{InnerReceiver, InnerSender, OuterReceiver, OuterSender, ALPN_PRIM};

pub type NewConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewConnectionHandler> + Send + Sync + 'static>;
pub type NewTimeoutConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewTimeoutConnectionHandler> + Send + Sync + 'static>;
pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;
pub type IOSender = OuterSender;
pub type IOReceiver = OuterReceiver;
pub struct WrapInnerSender(pub InnerSender);
pub struct WrapInnerReceiver(pub InnerReceiver);
pub struct WrapOuterSender(pub OuterSender);
pub struct WrapOuterReceiver(pub OuterReceiver);

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Result<&T> {
        match self.0.get(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_any().downcast_ref::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter: {} not found", type_name::<T>())),
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Result<&mut T> {
        match self.0.get_mut(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_mut_any().downcast_mut::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter not found")),
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

/// a parameter struct passed to handler function to avoid repeated construction of some singleton variable.
pub struct HandlerParameters {
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

#[async_trait]
pub trait NewTimeoutConnectionHandler: Send + Sync + 'static {
    async fn handle(
        &mut self,
        io_channel: (IOSender, IOReceiver),
        timeout_channel_receiver: OuterReceiver,
    ) -> Result<()>;
}

impl GenericParameter for WrapOuterSender {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for WrapOuterReceiver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for WrapInnerSender {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for WrapInnerReceiver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone)]
pub struct ServerConfig {
    pub address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: usize,
    /// the client and server should be the same value.
    connection_idle_timeout: u64,
    max_bi_streams: usize,
    max_uni_streams: usize,
    max_sender_side_channel_size: usize,
    max_receiver_side_channel_size: usize,
}

pub struct ServerConfigBuilder {
    #[allow(unused)]
    pub address: Option<SocketAddr>,
    #[allow(unused)]
    pub cert: Option<rustls::Certificate>,
    #[allow(unused)]
    pub key: Option<rustls::PrivateKey>,
    #[allow(unused)]
    pub max_connections: Option<usize>,
    #[allow(unused)]
    pub connection_idle_timeout: Option<u64>,
    #[allow(unused)]
    pub max_bi_streams: Option<usize>,
    #[allow(unused)]
    pub max_uni_streams: Option<usize>,
    pub max_sender_side_channel_size: Option<usize>,
    pub max_receiver_side_channel_size: Option<usize>,
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
            max_sender_side_channel_size: None,
            max_receiver_side_channel_size: None,
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

    pub fn with_max_connections(&mut self, max_connections: usize) -> &mut Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn with_connection_idle_timeout(&mut self, connection_idle_timeout: u64) -> &mut Self {
        self.connection_idle_timeout = Some(connection_idle_timeout);
        self
    }

    pub fn with_max_bi_streams(&mut self, max_bi_streams: usize) -> &mut Self {
        self.max_bi_streams = Some(max_bi_streams);
        self
    }

    pub fn with_max_uni_streams(&mut self, max_uni_streams: usize) -> &mut Self {
        self.max_uni_streams = Some(max_uni_streams);
        self
    }

    pub fn with_max_sender_side_channel_size(
        &mut self,
        max_sender_side_channel_size: usize,
    ) -> &mut Self {
        self.max_sender_side_channel_size = Some(max_sender_side_channel_size);
        self
    }

    pub fn with_max_receiver_side_channel_size(
        &mut self,
        max_receiver_side_channel_size: usize,
    ) -> &mut Self {
        self.max_receiver_side_channel_size = Some(max_receiver_side_channel_size);
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
        let max_sender_side_channel_size = self
            .max_sender_side_channel_size
            .ok_or_else(|| anyhow!("max_io_channel_size is required"))?;
        let max_receiver_side_channel_size = self
            .max_receiver_side_channel_size
            .ok_or_else(|| anyhow!("max_task_channel_size is required"))?;
        Ok(ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
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
        // deconstruct ServerConfig
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
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
        quinn_server_config.concurrent_connections(max_connections as u32);
        quinn_server_config.use_retry(true);
        // set quic transport parameters
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
            // the keep-alive interval should set on client.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let handler = generator();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    conn,
                    handler,
                    max_sender_side_channel_size,
                    max_receiver_side_channel_size,
                )
                .await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn handle_new_connection(
        mut conn: NewConnection,
        mut handler: Box<dyn NewConnectionHandler>,
        max_sender_side_channel_size: usize,
        max_receiver_side_channel_size: usize,
    ) -> Result<()> {
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) = async_channel::bounded(max_sender_side_channel_size);
        tokio::spawn(async move {
            _ = handler.handle((io_sender, io_receiver)).await;
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
                            let (bridge_sender, bridge_receiver) = (bridge_sender.clone(), bridge_receiver.clone());
                            let quickly_close_sender = quickly_close.0.clone();
                            tokio::spawn(async move {
                                let mut buf: Box<[u8; HEAD_LEN]> = Box::new([0_u8; HEAD_LEN]);
                                loop {
                                    select! {
                                        msg = MsgIOUtil::recv_msg(&mut buf, &mut io_streams.1) => {
                                            match msg {
                                                Ok(msg) => {
                                                    if let Err(_) = bridge_sender.send(msg).await {
                                                        break;
                                                    }
                                                },
                                                Err(_) => {
                                                    break;
                                                },
                                            }
                                        },
                                        msg = bridge_receiver.recv() => {
                                            match msg {
                                                Ok(msg) => {
                                                    if let Err(_) = MsgIOUtil::send_msg(msg, &mut io_streams.0).await {
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

pub struct ServerTimeout {
    config: Option<ServerConfig>,
    timeout: Duration,
}

impl ServerTimeout {
    pub fn new(config: ServerConfig, timeout: Duration) -> Self {
        Self {
            config: Some(config),
            timeout,
        }
    }

    pub async fn run(&mut self, generator: NewTimeoutConnectionHandlerGenerator) -> Result<()> {
        // deconstruct Server
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
            max_uni_streams,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
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
        quinn_server_config.concurrent_connections(max_connections as u32);
        quinn_server_config.use_retry(true);
        // set quic transport parameters
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .max_concurrent_uni_streams(quinn::VarInt::from_u64(max_uni_streams as u64).unwrap())
            // the keep-alive interval should set on client.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout as u64).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let handler = generator();
            let timeout = self.timeout;
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    conn,
                    handler,
                    max_sender_side_channel_size,
                    max_receiver_side_channel_size,
                    timeout,
                )
                .await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn handle_new_connection(
        mut conn: NewConnection,
        mut handler: Box<dyn NewTimeoutConnectionHandler>,
        max_sender_side_channel_size: usize,
        max_receiver_side_channel_size: usize,
        timeout: Duration,
    ) -> Result<()> {
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) = async_channel::bounded(max_sender_side_channel_size);
        let (timeout_sender, timeout_receiver) =
            tokio::sync::mpsc::channel(max_receiver_side_channel_size);
        tokio::spawn(async move {
            let _ = handler
                .handle((io_sender, io_receiver), timeout_receiver)
                .await;
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
                        if let Ok(io_streams) = io_streams {
                            let quickly_close_sender = quickly_close.0.clone();
                            let (bridge_sender, bridge_receiver) = (bridge_sender.clone(), bridge_receiver.clone());
                            let mut msg_io_timeout =
                                MsgIOTimeoutUtil::new(io_streams, timeout, max_receiver_side_channel_size, Some(AHashSet::from_iter(vec![Type::Ack, Type::Auth])));
                            let mut timeout_channel_receiver = msg_io_timeout.timeout_channel_receiver();
                            let timeout_sender = timeout_sender.clone();
                            tokio::spawn(async move {
                                loop {
                                    select! {
                                        msg = msg_io_timeout.recv_msg() => {
                                            match msg {
                                                Ok(msg) => {
                                                    let res = bridge_sender.send(msg).await;
                                                    if res.is_err() {
                                                        break;
                                                    }
                                                },
                                                Err(_) => {
                                                    break;
                                                }
                                            }
                                        },
                                        msg = bridge_receiver.recv() => {
                                            match msg {
                                                Ok(msg) => {
                                                    let res = msg_io_timeout.send_msg(msg).await;
                                                    if res.is_err() {
                                                        break;
                                                    }
                                                },
                                                Err(_) => {
                                                    let _ = quickly_close_sender.send(()).await;
                                                    break;
                                                },
                                            }
                                        },
                                        msg = timeout_channel_receiver.recv() => {
                                            match msg {
                                                Some(msg) => {
                                                    let res = timeout_sender.send(msg).await;
                                                    if res.is_err() {
                                                        break;
                                                    }
                                                },
                                                None => {
                                                    break;
                                                }
                                            }
                                        },
                                    }
                                }
                            });
                        } else {
                            println!("error: {}", io_streams.unwrap_err());
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

pub struct Server2 {
    config: Option<ServerConfig>,
}

impl Server2 {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub async fn run(&mut self, generator: NewConnectionHandlerGenerator) -> Result<()> {
        let ServerConfig {
            address,
            cert,
            key,
            max_sender_side_channel_size,
            max_receiver_side_channel_size,
            connection_idle_timeout,
            max_connections,
            ..
        } = self.config.take().unwrap();
        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        config.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let connection_counter = Arc::new(AtomicUsize::new(0));
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind(address).await?;
        while let Ok((stream, addr)) = listener.accept().await {
            info!("new connection: {}", addr);
            let tls_stream = acceptor.accept(stream).await?;
            let handler = generator();
            let number = connection_counter.fetch_add(1, Ordering::SeqCst);
            if number > max_connections {
                let (_reader, mut writer) = split(tls_stream);
                writer.write_all(b"too many connections.").await?;
                writer.flush().await?;
                writer.shutdown().await?;
                error!("too many connections.");
                continue;
            }
            let counter = connection_counter.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    tls_stream,
                    handler,
                    max_sender_side_channel_size,
                    max_receiver_side_channel_size,
                    counter,
                    connection_idle_timeout,
                )
                .await;
            });
        }
        Ok(())
    }

    async fn handle_new_connection(
        stream: TlsStream<TcpStream>,
        mut handler: Box<dyn NewConnectionHandler>,
        max_sender_side_channel_size: usize,
        max_receiver_side_channel_size: usize,
        connection_counter: Arc<AtomicUsize>,
        connection_idle_timeout: u64
    ) -> Result<()> {
        let (bridge_sender, io_receiver) =
            tokio::sync::mpsc::channel(max_receiver_side_channel_size);
        let (io_sender, bridge_receiver) = async_channel::bounded(max_sender_side_channel_size);
        tokio::spawn(async move {
            _ = handler.handle((io_sender, io_receiver)).await;
        });
        let (mut reader, mut writer) = split(stream);
        let mut buf: Box<[u8; HEAD_LEN]> = Box::new([0_u8; HEAD_LEN]);
        let idle_timeout = Duration::from_millis(connection_idle_timeout);
        let timer = tokio::time::sleep(idle_timeout);
        tokio::pin!(timer);
        loop {
            select! {
                msg = MsgIOUtil::recv_msg2(&mut buf, &mut reader) => {
                    timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                    match msg {
                        Ok(msg) => {
                            if msg.typ() == Type::Ping {
                                continue;
                            }
                            if let Err(_) = bridge_sender.send(msg).await {
                                break;
                            }
                        },
                        Err(_) => {
                            break;
                        },
                    }
                },
                msg = bridge_receiver.recv() => {
                    match msg {
                        Ok(msg) => {
                            if let Err(_) = MsgIOUtil::send_msg2(msg, &mut writer).await {
                                break;
                            }
                        },
                        Err(_) => {
                            break;
                        },
                    }
                    timer.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                },
                _ = &mut timer => {
                    error!("connection idle timeout.");
                    break;
                },
            }
        }
        debug!("connection closed.");
        connection_counter.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}
