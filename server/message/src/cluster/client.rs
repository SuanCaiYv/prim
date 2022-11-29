use std::{net::SocketAddr, sync::Arc};

use lib::{
    net::client::{ClientConfigBuilder, ClientMultiConnection, SubConnectionConfig},
    util::default_bind_ip, entity::{ServerInfo, ServerType, ServerStatus, Type, Msg},
    Result,
};
use tracing::error;
use anyhow::anyhow;

use crate::{config::CONFIG, util::my_id};

use super::get_cluster_sender_timeout_receiver_map;

pub(super) struct Client {
    multi_client: ClientMultiConnection,
}

impl Client {
    pub(super) fn new() -> Self {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(default_bind_ip())
            .with_domain(CONFIG.server.domain.clone())
            .with_cert(CONFIG.server.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let client_config = client_config.build().unwrap();
        let multi_client = ClientMultiConnection::new(client_config).unwrap();
        Self { multi_client }
    }

    pub(super) async fn new_connection(&self, remote_address: SocketAddr) -> Result<()> {
        let cluster_map = get_cluster_sender_timeout_receiver_map();
        let sub_config = SubConnectionConfig {
            remote_address,
            domain: CONFIG.server.domain.clone(),
            opened_bi_streams_number: CONFIG.transport.max_bi_streams,
            opened_uni_streams_number: CONFIG.transport.max_uni_streams,
            timeout: std::time::Duration::from_millis(3000),
        };
        let mut conn = self.multi_client.new_timeout_connection(sub_config).await?;
        let (io_sender, mut io_receiver, timeout_receiver) = conn.operation_channel();
        let server_info = ServerInfo {
            id: my_id(),
            address: CONFIG.server.cluster_address,
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MessageCluster,
            load: None,
        };
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        io_sender.send(Arc::new(auth)).await?;
        match io_receiver.recv().await {
            Some(res_msg) => {
                if res_msg.typ() != Type::Auth {
                    error!("auth failed");
                    return Err(anyhow!("auth failed"));
                }
                let res_server_info = ServerInfo::from(res_msg.payload());
                cluster_map.0.insert(res_server_info.id, io_sender.clone());
            }
            None => {
                error!("cluster client io_receiver recv None");
                return Err(anyhow!("cluster client io_receiver closed"))
            }
        }
        tokio::spawn(async move {
            // extend lifetime of connection
            let _conn = conn;
            if let Err(e) =
                super::handler::handler_func((io_sender, io_receiver), timeout_receiver).await
            {
                error!("handler_func error: {}", e);
            }
        });
        Ok(())
    }
}
