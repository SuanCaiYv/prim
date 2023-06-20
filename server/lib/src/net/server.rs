use std::{
    any::type_name,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::Waker,
    time::Duration,
};

use crate::{
    entity::{Msg, ReqwestMsg},
    net::{
        MsgIOTimeoutWrapper, MsgIOTlsServerTimeoutWrapper, NewReqwestConnectionHandler0,
        ReqwestMsgIOUtil, ReqwestOperator, ResponsePlaceholder,
    },
    Result,
};
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use futures_util::{pin_mut, FutureExt, StreamExt};
use quinn::{NewConnection, RecvStream, SendStream};
use tokio::{io::split, net::TcpStream};
use tokio::{io::AsyncWriteExt, sync::mpsc};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info};

use super::{
    InnerStates, MsgIOWrapper, MsgSender, Reqwest, ReqwestHandlerGenerator,
    ReqwestHandlerGenerator0, ReqwestOperatorManager, ALPN_PRIM,
};

pub type NewConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewConnectionHandler> + Send + Sync + 'static>;
pub type NewTimeoutConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewTimeoutConnectionHandler> + Send + Sync + 'static>;
pub type NewServerTimeoutConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewServerTimeoutConnectionHandler> + Send + Sync + 'static>;

pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);
#[derive(Clone)]
pub struct ReqwestCaller(pub Arc<ReqwestOperatorManager>);

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

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// the [`msg`] can be modified before clone() has been called.
    /// so each handler modifying [`msg`] should be put on the top of the handler list.
    async fn run(
        &self,
        msg: &mut Arc<Msg>,
        // this one contains some states corresponding to the quic stream.
        states: &mut InnerStates,
    ) -> Result<Msg>;
}

#[async_trait]
pub trait NewConnectionHandler: Send + Sync + 'static {
    /// to make the project more readable, we choose to use channel as io connector
    /// but to get better performance, directly send/recv from stream maybe introduced in future.
    async fn handle(&mut self, io_operators: MsgIOWrapper) -> Result<()>;
}

#[async_trait]
pub trait NewTimeoutConnectionHandler: Send + Sync + 'static {
    async fn handle(&mut self, io_operators: MsgIOTimeoutWrapper) -> Result<()>;
}

#[async_trait]
pub trait NewServerTimeoutConnectionHandler: Send + Sync + 'static {
    async fn handle(&mut self, io_operators: MsgIOTlsServerTimeoutWrapper) -> Result<()>;
}

impl GenericParameter for MsgSender {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GenericParameter for ReqwestCaller {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ReqwestCaller {
    pub fn call(&self, req: ReqwestMsg) -> Reqwest {
        self.0.call(req)
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub(crate) address: SocketAddr,
    pub(crate) cert: rustls::Certificate,
    pub(crate) key: rustls::PrivateKey,
    pub(crate) max_connections: usize,
    /// the client and server should be the same value.
    pub(crate) connection_idle_timeout: u64,
    pub(crate) max_bi_streams: usize,
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
        Ok(ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
        })
    }
}

/// use for client-server communication
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
            // the keep-alive interval should set on client.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, generator).await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn handle_new_connection(
        mut conn: NewConnection,
        generator: Arc<NewConnectionHandlerGenerator>,
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

/// use for server-server communication
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
            // the keep-alive interval should set on client.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout as u64).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let timeout = self.timeout;
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, generator, timeout).await;
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
                    let io_operators =
                        MsgIOTimeoutWrapper::new(io_streams.0, io_streams.1, timeout, None);
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
    timeout: Duration,
}

impl ServerTls {
    pub fn new(config: ServerConfig, timeout: Duration) -> Self {
        Self {
            config: Some(config),
            timeout,
        }
    }

    pub async fn run(
        &mut self,
        generator: NewServerTimeoutConnectionHandlerGenerator,
    ) -> Result<()> {
        let ServerConfig {
            address,
            cert,
            key,
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
            info!("new connection: {}", addr);
            let counter = connection_counter.clone();
            let timeout = self.timeout;
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    tls_stream,
                    handler,
                    counter,
                    timeout,
                    connection_idle_timeout,
                )
                .await;
            });
        }
        Ok(())
    }

    async fn handle_new_connection(
        stream: TlsStream<TcpStream>,
        mut handler: Box<dyn NewServerTimeoutConnectionHandler>,
        connection_counter: Arc<AtomicUsize>,
        timeout: Duration,
        connection_idle_timeout: u64,
    ) -> Result<()> {
        let (reader, writer) = split(stream);
        let idle_timeout = Duration::from_millis(connection_idle_timeout);
        let io_operators =
            MsgIOTlsServerTimeoutWrapper::new(writer, reader, timeout, idle_timeout, None);
        _ = handler.handle(io_operators).await;
        debug!("connection closed.");
        connection_counter.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}

pub(self) struct ServerReqwest0 {
    config: Option<ServerConfig>,
}

impl ServerReqwest0 {
    pub(self) fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub(self) async fn run(&mut self, generator: ReqwestHandlerGenerator0) -> Result<()> {
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
        } = self.config.take().unwrap();
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        quinn_server_config.concurrent_connections(max_connections as u32);
        quinn_server_config.use_retry(true);
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, generator).await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    #[inline(always)]
    async fn handle_new_connection(
        mut conn: NewConnection,
        generator: Arc<ReqwestHandlerGenerator0>,
    ) -> Result<()> {
        let mut client_caller = ReqwestOperatorManager::new();
        client_caller.target_mask = 0xF000_0000_0000_0000;
        let caller = Arc::new(client_caller);
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
                    let caller = caller.clone();
                    tokio::spawn(async move {
                        info!("new streams");
                        _ = handler.handle(io_streams, Some(caller)).await;
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

pub struct ServerReqwest {
    server: ServerReqwest0,
    timeout: Duration,
}

impl ServerReqwest {
    pub fn new(config: ServerConfig, timeout: Duration) -> Self {
        Self {
            server: ServerReqwest0::new(config),
            timeout,
        }
    }

    pub async fn run(&mut self, generator: Arc<ReqwestHandlerGenerator>) -> Result<()> {
        struct Generator0 {
            generator: Arc<ReqwestHandlerGenerator>,
            timeout: Duration,
        }

        #[async_trait]
        impl NewReqwestConnectionHandler0 for Generator0 {
            async fn handle(
                &mut self,
                msg_streams: (SendStream, RecvStream),
                client_caller: Option<Arc<ReqwestOperatorManager>>,
            ) -> Result<Option<ReqwestOperator>> {
                let (mut send_stream, mut recv_stream) = msg_streams;
                let (sender, mut receiver) = mpsc::channel::<(
                    ReqwestMsg,
                    Option<(u64, Arc<ResponsePlaceholder>, Waker)>,
                )>(16384);
                let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
                let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);

                let resp_waker_map0 = Arc::new(DashMap::new());
                let (tx, mut rx) = mpsc::channel::<u64>(4096);
                let stream_id = recv_stream.id().0;
                let sender_clone = sender.clone();
                let timeout = self.timeout;

                tokio::spawn(async move {
                    let waker_map = resp_waker_map0.clone();

                    let task1 = async {
                        loop {
                            match receiver.recv().await {
                                Some((req, external)) => match external {
                                    // a request from server
                                    Some((req_id, sender, waker)) => {
                                        waker_map.insert(req_id, (waker, sender));
                                        let res =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream)
                                                .await;
                                        let tx = tx.clone();
                                        tokio::spawn(async move {
                                            tokio::time::sleep(timeout).await;
                                            _ = tx.send(req_id).await;
                                        });
                                        if let Err(e) = res {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                    // a response from server
                                    None => {
                                        if let Err(e) =
                                            ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await
                                        {
                                            error!("send msg error: {}", e.to_string());
                                            break;
                                        }
                                    }
                                },
                                None => {
                                    debug!("receiver closed.");
                                    _ = send_stream.finish().await;
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let waker_map = resp_waker_map0.clone();

                    let task2 = async {
                        loop {
                            match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                                Ok(msg) => {
                                    let req_id = msg.req_id();
                                    // a response from client
                                    if req_id ^ 0xF000_0000_0000_0000 == 0 {
                                        match waker_map.remove(&req_id) {
                                            Some(waker) => {
                                                waker.1 .0.wake();
                                                _ = waker.1 .1.set(Ok(msg));
                                            }
                                            None => {
                                                error!("req_id: {} not found.", req_id)
                                            }
                                        }
                                    } else {
                                        // a request from client
                                        _ = msg_sender_outer.send(msg).await;
                                    }
                                }
                                Err(e) => {
                                    _ = recv_stream.stop(0u32.into());
                                    debug!("recv msg error: {}", e.to_string());
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let waker_map = resp_waker_map0;

                    let task3 = async {
                        loop {
                            match rx.recv().await {
                                Some(timeout_id) => match waker_map.remove(&timeout_id) {
                                    Some(waker) => {
                                        waker.1 .0.wake();
                                        _ = waker.1 .1.set(Err(anyhow!(
                                            "{:06} timeout: {}",
                                            stream_id,
                                            timeout_id
                                        )));
                                    }
                                    None => {}
                                },
                                None => {
                                    debug!("rx closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    let task4 = async {
                        loop {
                            match msg_receiver_inner.recv().await {
                                Some(msg) => {
                                    let res = sender_clone.send((msg, None)).await;
                                    if let Err(e) = res {
                                        error!("send msg error: {}", e.to_string());
                                        break;
                                    }
                                }
                                None => {
                                    debug!("msg_receiver_inner closed.");
                                    break;
                                }
                            }
                        }
                    }
                    .fuse();

                    pin_mut!(task1, task2, task3, task4);

                    loop {
                        futures::select! {
                            _ = task1 => {},
                            _ = task2 => {},
                            _ = task3 => {},
                            _ = task4 => {},
                            complete => {
                                break;
                            }
                        }
                    }
                });

                let mut handler = (self.generator)();
                let caller = client_caller.unwrap();
                caller
                    .push_operator(ReqwestOperator(stream_id as u16, sender))
                    .await;
                handler.set_reqwest_caller(ReqwestCaller(caller));
                handler
                    .handle((msg_sender_inner, msg_receiver_outer))
                    .await
                    .map_err(|e| {
                        error!("handler error: {}", e.to_string());
                        e
                    })?;
                Ok(None)
            }
        }

        let timeout = self.timeout;
        let generator0: ReqwestHandlerGenerator0 = Box::new(move || {
            Box::new(Generator0 {
                generator: generator.clone(),
                timeout,
            })
        });
        self.server.run(generator0).await
    }
}
