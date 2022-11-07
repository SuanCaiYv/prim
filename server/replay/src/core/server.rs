use crate::config::CONFIG;
use crate::core::handler::handle_new_connection;
use common::net::{InnerSender, ALPN_PRIM};
use common::Result;
use futures_util::StreamExt;
use std::sync::Arc;
use tracing::{error, info};

pub(crate) struct Server {}

impl Server {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn run(&mut self, global_sender: InnerSender) -> Result<()> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![CONFIG.server.cert.clone()], CONFIG.server.key.clone())?;
        // set custom alpn protocol
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        // set max concurrent connections
        quinn_server_config.concurrent_connections(CONFIG.server.max_connections);
        quinn_server_config.use_retry(true);
        // set quic transport parameters
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(CONFIG.transport.max_bi_streams)
            .max_concurrent_uni_streams(CONFIG.transport.max_uni_streams)
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                CONFIG.transport.connection_idle_timeout,
            )));
        let (endpoint, mut incoming) =
            quinn::Endpoint::server(quinn_server_config, CONFIG.server.address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let global_sender = global_sender.clone();
            tokio::spawn(async move {
                let res = handle_new_connection(conn, global_sender).await;
                if let Err(e) = res {
                    error!("error handling new connection: {}", e);
                }
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}
