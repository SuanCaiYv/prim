use std::sync::Arc;

use crate::config::CONFIG;
use crate::core::{get_ack_map, AckMap};
use anyhow::anyhow;
use common::entity::{Msg, ReplayMode, ServerInfo, ServerType};
use common::net::{LenBuffer, MsgIO, ALPN_PRIM};
use common::Result;
use futures_util::StreamExt;
use quinn::NewConnection;
use tracing::{debug, error, info};

pub(crate) struct Server {}

impl Server {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![CONFIG.server.cert.clone()], CONFIG.server.key.clone())?;
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        quinn_server_config.concurrent_connections(CONFIG.server.max_connections);
        quinn_server_config.use_retry(true);
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(CONFIG.transport.max_bi_streams)
            .max_concurrent_uni_streams(CONFIG.transport.max_uni_streams)
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                CONFIG.transport.connection_idle_timeout,
            )));
        let (endpoint, mut incoming) =
            quinn::Endpoint::server(quinn_server_config, CONFIG.server.inner_address)?;
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            tokio::spawn(async move {
                let res = handle_new_connection(conn).await;
                if let Err(e) = res {
                    error!("error handling new connection: {}", e);
                }
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}

pub(crate) async fn handle_new_connection(mut conn: NewConnection) -> Result<()> {
    let recv = conn.uni_streams.next().await;
    if recv.is_none() {
        return Err(anyhow!("no uni stream"));
    }
    let mut recv = recv.unwrap()?;
    let mut buffer: Box<LenBuffer> = Box::new([0_u8; 4]);
    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await?;
    let server_info = ServerInfo::from(msg.payload());
    match server_info.typ {
        ServerType::ReplayCluster => {}
        _ => {
            return Err(anyhow!("invalid server type"));
        }
    }
    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await?;
    let text = String::from_utf8_lossy(msg.payload()).to_string();
    info!("replay cluster: {} from {}", text, server_info.address);
    let ack_map = get_ack_map();
    tokio::spawn(async move {
        loop {
            let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
            if let Ok(msg) = msg {
                let res = msg_handler(msg, &ack_map).await;
                if res.is_err() {
                    error!("error handling msg: {}", res.unwrap_err());
                }
            } else {
                error!("error reading msg from cluster");
                break;
            }
        }
    });
    Ok(())
}

pub(self) async fn msg_handler(msg: Arc<Msg>, ack_map: &AckMap) -> Result<()> {
    let mode_value = String::from_utf8_lossy(msg.extension()).parse::<u8>()?;
    let mode = ReplayMode::from(mode_value);
    let replay_id = String::from_utf8_lossy(msg.payload()).to_string();
    match mode {
        ReplayMode::Cluster => {
            let res = ack_map.remove(&replay_id);
            if res.is_none() {
                debug!("replay_id: {} may not be here.", replay_id);
            }
            Ok(())
        }
        _ => Err(anyhow!("invalid replay mode")),
    }
}
