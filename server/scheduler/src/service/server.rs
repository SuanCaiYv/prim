use std::{sync::Arc, time::Duration};

use crate::{config::CONFIG, util::my_id};
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, Type},
    net::{
        server::{
            NewTimeoutConnectionHandler, NewTimeoutConnectionHandlerGenerator, ServerConfigBuilder,
            ServerTimeout,
        },
        MsgIOTimeoutServerWrapper,
    },
    Result, MESSAGE_NODE_ID_BEGINNING, RECORDER_NODE_ID_BEGINNING, SCHEDULER_NODE_ID_BEGINNING,
};

use anyhow::anyhow;
use async_trait::async_trait;
use tracing::error;

use super::{
    get_client_connection_map, get_message_node_set, get_recorder_node_set, get_scheduler_node_set,
    get_server_info_map,
};

pub(self) struct ClientConnectionHandler {}

impl ClientConnectionHandler {
    pub(self) fn new() -> ClientConnectionHandler {
        ClientConnectionHandler {}
    }
}

#[async_trait]
impl NewTimeoutConnectionHandler for ClientConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTimeoutServerWrapper) -> Result<()> {
        let (mut auth, sender, receiver, timeout) = io_operators.channels();
        let client_map = get_client_connection_map().0;
        let server_info_map = get_server_info_map().0;
        let message_node_set = get_message_node_set().0;
        let scheduler_node_set = get_scheduler_node_set().0;
        let recorder_node_set = get_recorder_node_set().0;
        match auth.recv().await {
            Some(auth_msg) => {
                if auth_msg.typ() != Type::Auth {
                    return Err(anyhow!("auth failed"));
                }
                let server_info = ServerInfo::from(auth_msg.payload());
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
                    typ: server_info.typ,
                    load: None,
                };
                let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
                res_msg.set_type(Type::Auth);
                res_msg.set_sender(my_id() as u64);
                res_msg.set_receiver(server_info.id as u64);
                sender.send(Arc::new(res_msg)).await?;
                client_map.insert(server_info.id, sender.clone());
                server_info_map.insert(server_info.id, server_info);
                if server_info.id >= MESSAGE_NODE_ID_BEGINNING
                    && server_info.id < SCHEDULER_NODE_ID_BEGINNING
                {
                    message_node_set.insert(server_info.id);
                } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING
                    && server_info.id < RECORDER_NODE_ID_BEGINNING
                {
                    scheduler_node_set.insert(server_info.id);
                } else if server_info.id >= RECORDER_NODE_ID_BEGINNING {
                    recorder_node_set.insert(server_info.id);
                }
                super::handler::handler_func(sender, receiver, timeout, &server_info).await?;
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
            .with_address(CONFIG.server.service_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams);
        let server_config = server_config_builder.build().unwrap();
        // todo("timeout set")!
        let mut server = ServerTimeout::new(server_config, Duration::from_millis(3000));
        let generator: NewTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(ClientConnectionHandler::new()));
        server.run(generator).await?;
        Ok(())
    }
}
