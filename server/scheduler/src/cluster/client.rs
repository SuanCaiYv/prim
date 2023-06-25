use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use anyhow::anyhow;
use lib_tokio::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::{
        client::{ClientConfigBuilder, ClientReqwest},
        NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator, ReqwestHandlerMap,
    },
    Result,
};

use crate::{
    cluster::handler::{logic, message},
    config::CONFIG,
    util::my_id,
};

use super::{get_cluster_connection_set, server::ClientConnectionHandler};
pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let cluster_set = get_cluster_connection_set();
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
            if cluster_set.contains(&addr) {
                continue;
            }
            let ipv4 = CONFIG.server.cluster_address.is_ipv4();
            let mut client_config = ClientConfigBuilder::default();
            client_config
                .with_remote_address(addr.to_owned())
                .with_ipv4_type(ipv4)
                .with_domain(CONFIG.server.domain.clone())
                .with_cert(CONFIG.cluster.cert.clone())
                .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
                .with_max_bi_streams(CONFIG.transport.max_bi_streams);
            let client_config = client_config.build().unwrap();

            let mut service_address = CONFIG.server.service_address;
            service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
            let mut cluster_address = CONFIG.server.cluster_address;
            cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
            let server_info = ServerInfo {
                id: my_id(),
                service_address,
                cluster_address: Some(cluster_address),
                connection_id: 0,
                status: ServerStatus::Online,
                typ: ServerType::SchedulerCluster,
                load: None,
            };

            let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
            handler_map.insert(
                ReqwestResourceID::NodeAuth.value(),
                Box::new(logic::ClientAuth {}),
            );
            handler_map.insert(
                ReqwestResourceID::MessageNodeRegister.value(),
                Box::new(message::NodeRegister {}),
            );
            handler_map.insert(
                ReqwestResourceID::MessageNodeUnregister.value(),
                Box::new(message::NodeUnregister {}),
            );
            let handler_map = ReqwestHandlerMap::new(handler_map);
            let generator: ReqwestHandlerGenerator =
                Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                    Box::new(ClientConnectionHandler::new(handler_map.clone()))
                });

            let generator = Arc::new(generator);
            let mut client = ClientReqwest::new(client_config, Duration::from_millis(3000));
            let operator = client.build(generator).await?;

            let auth_msg = ReqwestMsg::with_resource_id_payload(
                ReqwestResourceID::NodeAuth.value(),
                &server_info.to_bytes(),
            );
            let resp = operator.call(auth_msg).await?;
            if resp.payload() != b"true" {
                return Err(anyhow!("auth failed"));
            }
        }
        Ok(())
    }
}
