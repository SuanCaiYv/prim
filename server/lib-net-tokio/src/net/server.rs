use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::Waker,
    time::Duration,
};

use super::{
    MsgIOWrapper, MsgSender, NewReqwestConnectionHandler, Reqwest, ReqwestHandlerGenerator,
    ReqwestHandlerGenerator0, ReqwestOperatorManager,
};
use crate::net::{
    MsgIOWrapperTcpS, NewReqwestConnectionHandler0, ReqwestMsgIOUtil, ReqwestMsgIOWrapperTcpS,
    ReqwestOperator, ResponsePlaceholder,
};

use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{pin_mut, FutureExt};
use lib::{
    entity::ReqwestMsg,
    net::{server::ServerConfig, GenericParameter, ALPN_PRIM},
    Result,
};
use quinn::{Connection, RecvStream, SendStream};
use tokio::{
    io::{split, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info};

pub type NewConnectionHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewConnectionHandler> + Send + Sync + 'static>;
pub type NewConnectionHandlerGeneratorTcp =
    Box<dyn Fn() -> Box<dyn NewConnectionHandlerTcp> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct ReqwestCaller(pub Arc<ReqwestOperatorManager>);

#[async_trait]
pub trait NewConnectionHandler: Send + Sync + 'static {
    /// to make the project more readable, we choose to use channel as io connector
    /// but to get better performance, directly send/recv from stream maybe introduced in future.
    async fn handle(&mut self, io_operators: MsgIOWrapper) -> Result<()>;
}

#[async_trait]
pub trait NewConnectionHandlerTcp: Send + Sync + 'static {
    /// to make the project more readable, we choose to use channel as io connector
    /// but to get better performance, directly send/recv from stream maybe introduced in future.
    async fn handle(&mut self, io_operators: MsgIOWrapperTcpS) -> Result<()>;
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
        let endpoint = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = endpoint.accept().await {
            let conn = conn.await?;
            info!("new connection: {}", conn.remote_address().to_string());
            let generator = generator.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, generator).await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }

    async fn handle_new_connection(
        conn: Connection,
        generator: Arc<NewConnectionHandlerGenerator>,
    ) -> Result<()> {
        loop {
            match conn.accept_bi().await {
                Ok(io_streams) => {
                    let mut handler = generator();
                    let io_operators = MsgIOWrapper::new(io_streams.0, io_streams.1, 0);
                    tokio::spawn(async move {
                        _ = handler.handle(io_operators).await;
                    });
                }
                Err(e) => {
                    match e {
                        quinn::ConnectionError::ApplicationClosed { .. } => {
                            info!("the peer close the connection.");
                        }
                        quinn::ConnectionError::ConnectionClosed { .. } => {
                            info!("the peer close the connection but by quic.");
                        }
                        quinn::ConnectionError::Reset => {
                            error!("connection reset.");
                        }
                        quinn::ConnectionError::TransportError { .. } => {
                            error!("connect by fake specification.");
                        }
                        quinn::ConnectionError::TimedOut => {
                            error!("connection idle for too long time.");
                        }
                        quinn::ConnectionError::VersionMismatch => {
                            error!("connect by unsupported protocol version.");
                        }
                        quinn::ConnectionError::LocallyClosed => {
                            error!("local server fatal.");
                        }
                    }
                    break;
                }
            }
        }
        debug!("connection closed.");
        conn.close(0u32.into(), b"it's time to say goodbye.");
        Ok(())
    }
}

pub struct ServerTcp {
    config: Option<ServerConfig>,
}

impl ServerTcp {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub async fn run(&mut self, generator: NewConnectionHandlerGeneratorTcp) -> Result<()> {
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
            let number = connection_counter.fetch_add(1, Ordering::AcqRel);
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
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    tls_stream,
                    handler,
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
        mut handler: Box<dyn NewConnectionHandlerTcp>,
        connection_counter: Arc<AtomicUsize>,
        connection_idle_timeout: u64,
    ) -> Result<()> {
        let idle_timeout = Duration::from_millis(connection_idle_timeout);
        let io_operators = MsgIOWrapperTcpS::new(stream, idle_timeout, 0);
        _ = handler.handle(io_operators).await;
        debug!("connection closed.");
        connection_counter.fetch_sub(1, Ordering::AcqRel);
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
        let endpoint = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = endpoint.accept().await {
            let conn = conn.await?;
            info!("new connection: {}", conn.remote_address().to_string());
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
        conn: Connection,
        generator: Arc<ReqwestHandlerGenerator0>,
    ) -> Result<()> {
        let client_caller = ReqwestOperatorManager::new(0xF000_0000_0000_0000);
        let caller = Arc::new(client_caller);
        loop {
            match conn.accept_bi().await {
                Ok(io_streams) => {
                    let mut handler = generator();
                    let caller = caller.clone();
                    tokio::spawn(async move {
                        info!("new streams");
                        _ = handler.handle(io_streams, Some(caller)).await;
                    });
                }
                Err(e) => {
                    match e {
                        quinn::ConnectionError::ApplicationClosed { .. } => {
                            info!("the peer close the connection.");
                        }
                        quinn::ConnectionError::ConnectionClosed { .. } => {
                            info!("the peer close the connection but by quic.");
                        }
                        quinn::ConnectionError::Reset => {
                            error!("connection reset.");
                        }
                        quinn::ConnectionError::TransportError { .. } => {
                            error!("connect by fake specification.");
                        }
                        quinn::ConnectionError::TimedOut => {
                            error!("connection idle for too long time.");
                        }
                        quinn::ConnectionError::VersionMismatch => {
                            error!("connect by unsupported protocol version.");
                        }
                        quinn::ConnectionError::LocallyClosed => {
                            error!("local server fatal.");
                        }
                    }
                    break;
                }
            }
        }
        debug!("connection closed.");
        conn.close(0u32.into(), b"it's time to say goodbye.");
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
                                    if req_id & 0xF000_0000_0000_0000 != 0 {
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

pub struct ServerReqwestTcp {
    config: Option<ServerConfig>,
}

impl ServerReqwestTcp {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub async fn run(&mut self, generator: Arc<ReqwestHandlerGenerator>) -> Result<()> {
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
            let tls_stream = acceptor.accept(stream).await;
            if tls_stream.is_err() {
                error!("tls handshake failed.");
                continue;
            }
            let mut tls_stream = tls_stream.unwrap();
            let handler = generator();
            let number = connection_counter.fetch_add(1, Ordering::AcqRel);
            if number > max_connections {
                _ = tls_stream.write_all(b"too many connections.").await;
                tls_stream.flush().await?;
                tls_stream.shutdown().await?;
                error!("too many connections.");
                continue;
            }
            info!("new connection: {}", addr);
            let counter = connection_counter.clone();
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(
                    tls_stream,
                    handler,
                    counter,
                    connection_idle_timeout,
                )
                .await;
            });
        }
        Ok(())
    }

    #[inline(always)]
    async fn handle_new_connection(
        stream: TlsStream<TcpStream>,
        mut handler: Box<dyn NewReqwestConnectionHandler>,
        connection_counter: Arc<AtomicUsize>,
        connection_idle_timeout: u64,
    ) -> Result<()> {
        let idle_timeout = Duration::from_millis(connection_idle_timeout);
        let mut io_operators = ReqwestMsgIOWrapperTcpS::new(stream, idle_timeout);
        _ = handler.handle(io_operators.io_channels()).await;
        debug!("connection closed.");
        connection_counter.fetch_sub(1, Ordering::AcqRel);
        Ok(())
    }
}
