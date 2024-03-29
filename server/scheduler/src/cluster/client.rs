use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use anyhow::anyhow;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::client::ClientConfigBuilder,
    Result,
};
use lib_net_tokio::net::{
    client::ClientReqwest, NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator,
    ReqwestHandlerMap,
};

use crate::{
    cluster::handler::{logic, message},
    config::config,
    util::my_id,
};

use super::{get_cluster_connection_set, server::ClientConnectionHandler};
pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let cluster_set = get_cluster_connection_set();
        let mut addr_vec = config().cluster.addresses.clone();
        let my_addr = &config().server.cluster_address;
        addr_vec.sort();
        let num = (addr_vec.len() - 1) / 2;
        let mut index = 0;
        for addr in addr_vec.iter() {
            index += 1;
            if &addr.to_string() == my_addr {
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
            let mut client_config = ClientConfigBuilder::default();
            client_config
                .with_remote_address(addr.to_owned())
                .with_ipv4_type(config().server.ipv4)
                .with_domain(config().server.domain.clone())
                .with_cert(config().cluster.cert.clone())
                .with_keep_alive_interval(config().transport.keep_alive_interval)
                .with_max_bi_streams(config().transport.max_bi_streams);
            let client_config = client_config.build().unwrap();

            let server_info = ServerInfo {
                id: my_id(),
                service_address: config().server.service_address.clone(),
                cluster_address: Some(config().server.cluster_address.clone()),
                connection_id: 0,
                status: ServerStatus::Online,
                typ: ServerType::SchedulerCluster,
                load: None,
            };

            let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
            handler_map.insert(ReqwestResourceID::NodeAuth, Box::new(logic::ClientAuth {}));
            handler_map.insert(
                ReqwestResourceID::MessageNodeRegister,
                Box::new(message::NodeRegister {}),
            );
            handler_map.insert(
                ReqwestResourceID::MessageNodeUnregister,
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
                ReqwestResourceID::NodeAuth,
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
