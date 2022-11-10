use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::config::CONFIG;
use crate::core::get_cluster_connection_set;
use anyhow::anyhow;
use common::entity::{Msg, ServerInfo, ServerStatus, ServerType, Type};
use common::{
    net::{MsgIO, ALPN_PRIM},
    util::default_bind_ip,
    Result,
};
use quinn::{SendStream, VarInt};
use tracing::error;

pub(crate) struct Cluster {}

impl Cluster {
    pub(crate) async fn run() -> Result<()> {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let cluster_set = get_cluster_connection_set();
        for address in CONFIG.cluster_addresses.iter() {
            let mut send = Cluster::connect(*address).await?;
            let server_info = ServerInfo {
                id: 0,
                address: CONFIG.server.inner_address,
                connection_id: 0,
                status: ServerStatus::Online,
                typ: ServerType::ReplayCluster,
                load: None,
            };
            let mut msg = Msg::raw_payload(&server_info.to_bytes());
            msg.set_type(Type::Auth);
            msg.set_sender(server_info.id as u64);
            msg.set_sender_node(server_info.id);
            MsgIO::write_msg(Arc::new(msg), &mut send).await?;
            let text = format!("hello peer, I am {}", server_info.id);
            let msg = Msg::text(server_info.id as u64, 0, server_info.id, 0, text);
            MsgIO::write_msg(Arc::new(msg), &mut send).await?;
            let mut send_channel =
                tokio::sync::mpsc::channel(CONFIG.performance.max_sender_side_channel_size);
            cluster_set.insert(*address, send_channel.0);
            tokio::spawn(async move {
                loop {
                    let msg = send_channel.1.recv().await;
                    if let Some(msg) = msg {
                        let res = MsgIO::write_msg(msg, &mut send).await;
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

    async fn connect(remote_address: SocketAddr) -> Result<SendStream> {
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
        let send = connection.open_uni().await?;
        Ok(send)
    }
}
