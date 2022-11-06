use std::{net::SocketAddr, sync::Arc};

use super::CLUSTER_CONNECTION_SET;
use crate::config::CONFIG;
use anyhow::anyhow;
use common::{
    net::{MsgIO, ALPN_PRIM},
    util::default_bind_ip,
    Result,
};
use quinn::{RecvStream, SendStream, VarInt};
use tracing::error;

pub(crate) struct Cluster {}

impl Cluster {
    pub(crate) async fn run() -> Result<()> {
        let connection_set = CLUSTER_CONNECTION_SET.clone();
        for address in CONFIG.cluster_addresses.iter() {
            let io_streams = Cluster::connect(*address).await?;
            let (mut send_stream, _) = io_streams;
            let mut send_channel = tokio::sync::mpsc::channel(32);
            connection_set.insert(*address, send_channel.0);
            tokio::spawn(async move {
                loop {
                    let msg = send_channel.1.recv().await;
                    if let Some(msg) = msg {
                        let res = MsgIO::write_msg(msg, &mut send_stream).await;
                        if res.is_err() {
                            error!("error writing msg to cluster");
                            break;
                        }
                    } else {
                        error!("send channel closed");
                        break;
                    }
                }
            });
        }
        Ok(())
    }

    async fn connect(remote_address: SocketAddr) -> Result<(SendStream, RecvStream)> {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&CONFIG.server.cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = quinn::Endpoint::client(default_bind_ip())?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(VarInt::from_u32(1))
            .max_concurrent_uni_streams(VarInt::from_u32(0))
            .keep_alive_interval(Some(CONFIG.transport.keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, CONFIG.server.domain.as_str())?
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let io_streams = connection.open_bi().await?;
        Ok(io_streams)
    }
}
