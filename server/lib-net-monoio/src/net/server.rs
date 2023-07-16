use std::sync::atomic::Ordering;
use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use async_trait::async_trait;
use lib::{
    entity::ReqwestMsg,
    net::{server::ServerConfig, ALPN_PRIM},
    Result,
};
use local_sync::mpsc;
use monoio::net::TcpStream;
use monoio::{
    io::{AsyncWriteRent, AsyncWriteRentExt},
    net::TcpListener,
};
use monoio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info};

use crate::net::ReqwestMsgIOWrapper;

pub type ReqwestHandlerGenerator = Box<dyn Fn() -> Box<dyn NewReqwestConnectionHandler>>;

#[async_trait(?Send)]
pub trait NewReqwestConnectionHandler: 'static {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::bounded::Tx<ReqwestMsg>, mpsc::bounded::Rx<ReqwestMsg>),
    ) -> Result<()>;
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

    pub async fn run(&mut self, generator: ReqwestHandlerGenerator) -> Result<()> {
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
        let acceptor = TlsAcceptor::from(config);
        let listener = TcpListener::bind(address)?;
        while let Ok((stream, addr)) = listener.accept().await {
            let tls_stream = acceptor.accept(stream).await;
            if tls_stream.is_err() {
                error!("tls handshake failed.");
                continue;
            }
            let mut tls_stream = tls_stream.unwrap();
            let handler = generator();
            let number = connection_counter.fetch_add(1, Ordering::SeqCst);
            if number > max_connections {
                _ = tls_stream.write_all(b"too many connections.").await;
                tls_stream.flush().await?;
                tls_stream.shutdown().await?;
                error!("too many connections.");
                continue;
            }
            info!("new connection: {}", addr);
            let counter = connection_counter.clone();
            monoio::spawn(async move {
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
        let mut io_operators = ReqwestMsgIOWrapper::new(stream, idle_timeout);
        _ = handler.handle(io_operators.io_channels()).await;
        debug!("connection closed.");
        connection_counter.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}
