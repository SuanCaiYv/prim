use std::sync::Arc;

use crate::{config::CONFIG, util::my_id};
use lib::{
    net::server::{
        IOReceiver, IOSender, NewConnectionHandler, NewConnectionHandlerGenerator,
        ServerConfigBuilder,
    },
    Result, entity::{Type, ServerInfo, ServerStatus, Msg},
};
use async_trait::async_trait;
use anyhow::anyhow;
use tracing::error;

pub(self) struct MessageConnectionHandler {}

impl MessageConnectionHandler {
    pub(self) fn new() -> MessageConnectionHandler {
        MessageConnectionHandler {}
    }
}

#[async_trait]
impl NewConnectionHandler for MessageConnectionHandler {
    async fn handle(&mut self, mut io_channel: (IOSender, IOReceiver)) -> Result<()> {
        match io_channel.1.recv().await {
            Some(auth_msg) => {
                if auth_msg.typ() != Type::Auth {
                    return Err(anyhow!("auth failed"));
                }
                let server_info = ServerInfo::from(auth_msg.payload());
                let mut service_address = CONFIG.server.service_address;
                service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
                let res_server_info = ServerInfo {
                    id: my_id(),
                    service_address,
                    cluster_address: None,
                    connection_id: 0,
                    status: ServerStatus::Normal,
                    typ: server_info.typ,
                    load: None,
                };
                let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
                res_msg.set_type(Type::Auth);
                res_msg.set_sender(my_id() as u64);
                res_msg.set_receiver(server_info.id as u64);
                io_channel.0.send(Arc::new(res_msg)).await?;
                super::handler::handler_func(io_channel).await?;
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
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let server_config = server_config_builder.build().unwrap();
        // todo("timeout set")!
        let mut server = lib::net::server::Server::new(server_config);
        let generator: NewConnectionHandlerGenerator =
            Box::new(move || Box::new(MessageConnectionHandler::new()));
        server.run(generator).await?;
        Ok(())
    }
}
