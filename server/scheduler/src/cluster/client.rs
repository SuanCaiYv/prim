use std::{time::Duration, sync::Arc};

use lib::{
    entity::{ServerInfo, ServerStatus, ServerType, Type, Msg},
    net::client::{ClientConfigBuilder, ClientTimeout},
    Result,
};
use tracing::{error, debug};

use crate::{config::CONFIG, util::my_id};

use super::{get_cluster_connection_set, get_cluster_connection_map};
pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let cluster_set = get_cluster_connection_set();
        let cluster_map = get_cluster_connection_map();
        let mut addr_vec = CONFIG.cluster.addresses.clone();
        let my_addr = CONFIG.server.cluster_address;
        addr_vec.sort();
        let num = (addr_vec.len() - 1) / 2;
        let mut index = 0;
        for addr in addr_vec.iter() {
            index += 1;
            if *addr == my_addr {
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        for _ in 0..num {
            let i = index % addr_vec.len();
            index += 1;
            let addr = &addr_vec[i];
            if cluster_set.contains(addr) {
                continue;
            }
            let mut client_config = ClientConfigBuilder::default();
            client_config
                .with_remote_address(addr.to_owned())
                .with_domain(CONFIG.cluster.domain.clone())
                .with_cert(CONFIG.cluster.cert.clone())
                .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
                .with_max_bi_streams(CONFIG.transport.max_bi_streams)
                .with_max_uni_streams(CONFIG.transport.max_uni_streams)
                .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
                .with_max_receiver_side_channel_size(
                    CONFIG.performance.max_receiver_side_channel_size,
                );
            let client_config = client_config.build().unwrap();
            let mut client = ClientTimeout::new(client_config, Duration::from_millis(3000));
            client.run().await?;
            let (io_sender, mut io_receiver, timeout_receiver) = client.io_channel().await?;
            debug!("cluster client {} connected", addr);
            let server_info = ServerInfo {
                id: my_id(),
                address: my_addr,
                connection_id: 0,
                status: ServerStatus::Online,
                typ: ServerType::SchedulerCluster,
                load: None,
            };
            let mut auth = Msg::raw_payload(&server_info.to_bytes());
            auth.set_type(Type::Auth);
            auth.set_sender(server_info.id as u64);
            io_sender.send(Arc::new(auth)).await?;
            let res_server_info;
            match io_receiver.recv().await {
                Some(res_msg) => {
                    if res_msg.typ() != Type::Auth {
                        error!("auth failed");
                        continue;
                    }
                    res_server_info = ServerInfo::from(res_msg.payload());
                    cluster_set.insert(addr.to_owned());
                    cluster_map.0.insert(res_server_info.id, io_sender.clone());
                }
                None => {
                    error!("cluster client io_receiver recv None");
                    continue;
                }
            }
            debug!("start handler function of client.");
            tokio::spawn(async move {
                // try to extend the lifetime of client to avoid being dropped.
                let _client = client;
                if let Err(e) =
                    super::handler::handler_func((io_sender, io_receiver), timeout_receiver, &res_server_info).await
                {
                    error!("handler_func error: {}", e);
                }
            });
        }
        Ok(())
    }
}
