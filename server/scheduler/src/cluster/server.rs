use std::{sync::Arc, time::Duration};

use crate::{config::CONFIG, util::my_id, cluster::MsgSender};
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    net::{
        server::{
            NewTimeoutConnectionHandler, NewTimeoutConnectionHandlerGenerator, ServerConfigBuilder,
            ServerTimeout,
        },
        MsgIOTimeoutWrapper,
    },
    Result,
};

use anyhow::anyhow;
use async_trait::async_trait;
use tracing::{debug, error, info};

use super::{get_cluster_connection_map, get_cluster_connection_set};

pub(self) struct ClusterConnectionHandler {}

impl ClusterConnectionHandler {
    pub(self) fn new() -> ClusterConnectionHandler {
        ClusterConnectionHandler {}
    }
}

#[async_trait]
impl NewTimeoutConnectionHandler for ClusterConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTimeoutWrapper) -> Result<()> {
        let (sender, mut receiver, timeout) = io_operators.channels();
        let cluster_set = get_cluster_connection_set();
        let cluster_map = get_cluster_connection_map().0;
        match receiver.recv().await {
            Some(auth_msg) => {
                if auth_msg.typ() != Type::Auth {
                    return Err(anyhow!("auth failed"));
                }
                let server_info = ServerInfo::from(auth_msg.payload());
                info!("cluster server {} connected", server_info.id);
                let mut service_address = CONFIG.server.service_address;
                service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
                let mut cluster_address = CONFIG.server.cluster_address;
                cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
                let res_server_info = ServerInfo {
                    id: my_id(),
                    service_address,
                    cluster_address: Some(cluster_address),
                    connection_id: 0,
                    status: ServerStatus::Normal,
                    typ: ServerType::SchedulerCluster,
                    load: None,
                };
                let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
                res_msg.set_type(Type::Auth);
                res_msg.set_sender(my_id() as u64);
                res_msg.set_receiver(server_info.id as u64);
                sender.send(Arc::new(res_msg)).await?;
                cluster_set.insert(server_info.cluster_address.unwrap());
                cluster_map.insert(server_info.id, MsgSender::Server(sender.clone()));
                debug!("start handler function of server.");
                super::handler::handler_func(MsgSender::Server(sender), receiver, timeout, &server_info).await?;
                Ok(())
            }
            None => {
                error!("cannot receive auth message");
                Err(anyhow!("cannot receive auth message"))
            }
        }
    }
}

pub(crate) struct Server {}

impl Server {
    pub(crate) async fn run() -> Result<()> {
        let mut server_config_builder = ServerConfigBuilder::default();
        server_config_builder
            .with_address(CONFIG.server.cluster_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let server_config = server_config_builder.build().unwrap();
        // todo("timeout set")!
        let mut server = ServerTimeout::new(server_config, Duration::from_millis(3000));
        let generator: NewTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(ClusterConnectionHandler::new()));
        server.run(generator).await?;
        Ok(())
    }
}
