use super::Result;
use anyhow::anyhow;
use futures_util::StreamExt;
use quinn::VarInt;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::net::{ConnectionTask, ConnectionTaskGenerator, ALPN_PRIM};

#[allow(unused)]
pub struct ServerConfig {
    address: SocketAddr,
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
    max_connections: VarInt,
    /// should set only on clients.
    keep_alive_interval: VarInt,
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
    pub keep_alive_interval: Option<VarInt>,
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

    pub fn with_keep_alive_interval(&mut self, keep_alive_interval: VarInt) -> &mut Self {
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

pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

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
            // todo address migration and keep-alive.
            .max_idle_timeout(Some(quinn::IdleTimeout::from(connection_idle_timeout)));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let handler: Box<dyn ConnectionTask> = connection_task_generator(conn);
            tokio::spawn(async move {
                let _ = handler.handle().await;
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}
