use std::{
    any::type_name,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    entity::{Msg, Type, HEAD_LEN},
    net::{MsgIOTimeoutWrapper, MsgIOUtil, TinyMsgIOUtil},
    Result,
};
use ahash::{AHashMap, AHashSet};
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use quinn::{NewConnection, RecvStream, SendStream};
use tokio::io::AsyncWriteExt;
use tokio::{io::split, net::TcpStream, select};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info};

use super::{InnerReceiver, InnerSender, OuterReceiver, OuterSender, ALPN_PRIM, MsgIOWrapper};

pub type NewConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewConnectionHandler> + Send + Sync + 'static>;
pub type NewTimeoutConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewTimeoutConnectionHandler> + Send + Sync + 'static>;
pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;
pub type AuthFunc = Box<dyn Fn(&str) -> bool + Send + Sync + 'static>;
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
    async fn handle(&mut self, io_operators: MsgIOWrapper) -> Result<()>;
}

#[async_trait]
pub trait NewTimeoutConnectionHandler: Send + Sync + 'static {
    async fn handle(
        &mut self,
        io_operators: MsgIOTimeoutWrapper,
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
    auth_func: AuthFunc,
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
    pub auth_func: Option<AuthFunc>,
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
            auth_func: None,
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

    pub fn with_auth_func(&mut self, auth_func: AuthFunc) -> &mut Self {
        self.auth_func = Some(auth_func);
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
        let auth_func = self
            .auth_func
            .unwrap_or_else(|| Box::new(|_token: &str| true));
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
            auth_func,
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
            auth_func,
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
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let mut conn = conn.await?;
            if let Some(auth_stream) = conn.uni_streams.next().await {
                let mut auth_stream = auth_stream?;
                let auth_msg = TinyMsgIOUtil::recv_msg(&mut auth_stream).await?;
                let token = String::from_utf8_lossy(auth_msg.payload()).to_string();
                if !auth_func(&token) {
                    error!("auth failed");
                    continue;
                } else {
                    auth_stream.stop(quinn::VarInt::from_u32(0));
                }
            } else {
                error!("auth stream is not found");
                continue;
            }
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    conn,
                    generator,
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
        generator: Arc<NewConnectionHandlerGenerator>,
        max_sender_side_channel_size: usize,
        max_receiver_side_channel_size: usize,
    ) -> Result<()> {
        loop {
            if let Some(streams) = conn.bi_streams.next().await {
                let io_streams = match streams {
                    Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                        info!("the peer close the connection.");
                        Err(anyhow!("the peer close the connection."))
                    }
                    Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                        info!("the peer close the connection but by quic.");
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
                    let mut handler = generator();
                    let io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1);
                    tokio::spawn(async move {
                        _ = handler.handle(io_operators).await;
                    });
                } else {
                    break;
                }
            } else {
                break;
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
            auth_func,
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
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let mut conn = conn.await?;
            if let Some(auth_stream) = conn.uni_streams.next().await {
                let mut auth_stream = auth_stream?;
                let auth_msg = TinyMsgIOUtil::recv_msg(&mut auth_stream).await?;
                let token = String::from_utf8_lossy(auth_msg.payload()).to_string();
                if !auth_func(&token) {
                    error!("auth failed");
                    continue;
                } else {
                    auth_stream.stop(quinn::VarInt::from_u32(0));
                }
            } else {
                error!("auth stream is not found");
                continue;
            }
            let handler = generator();
            let timeout = self.timeout;
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    conn,
                    generator,
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
        generator: Arc<NewTimeoutConnectionHandlerGenerator>,
        timeout: Duration,
    ) -> Result<()> {
        loop {
            if let Some(streams) = conn.bi_streams.next().await {
                let io_streams = match streams {
                    Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                        info!("the peer close the connection.");
                        Err(anyhow!("the peer close the connection."))
                    }
                    Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                        info!("the peer close the connection but by quic.");
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
                    let io_operators = MsgIOTimeoutWrapper::new(
                        io_streams,
                        timeout,
                        Some(AHashSet::from_iter(vec![Type::Ack, Type::Auth])),
                        false,
                    );
                    let mut handler = generator();
                    tokio::spawn(async move {
                        _ = handler.handle(io_operators).await;
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        debug!("connection closed.");
        conn.connection
            .close(0u32.into(), b"it's time to say goodbye.");
        Ok(())
    }
}

pub struct ServerTls {
    config: Option<ServerConfig>,
}

impl ServerTls {
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
        // todo auth
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
        connection_idle_timeout: u64,
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
                msg = MsgIOUtil::recv_msg_server(&mut buf, &mut reader) => {
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
                            if let Err(_) = MsgIOUtil::send_msg_server(msg, &mut writer).await {
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
