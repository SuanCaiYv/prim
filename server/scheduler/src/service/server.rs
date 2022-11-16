use std::{sync::Arc, time::Duration};

use crate::{config::CONFIG, util::my_id};
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, Type},
    net::{
        server::{
            IOReceiver, IOSender, NewTimeoutConnectionHandler,
            NewTimeoutConnectionHandlerGenerator, ServerConfigBuilder, ServerTimeout,
        },
        OuterReceiver,
    },
    Result,
};

use anyhow::anyhow;
use async_trait::async_trait;
use tracing::error;

use super::get_client_sender_timeout_receiver_map;

pub(self) struct ClientConnectionHandler {}

impl ClientConnectionHandler {
    pub(self) fn new() -> ClientConnectionHandler {
        ClientConnectionHandler {}
    }
}

#[async_trait]
impl NewTimeoutConnectionHandler for ClientConnectionHandler {
    async fn handle(
        &mut self,
        mut io_channel: (IOSender, IOReceiver),
        timeout_channel_receiver: OuterReceiver,
    ) -> Result<()> {
        let client_map = get_client_sender_timeout_receiver_map();
        match io_channel.1.recv().await {
            Some(auth_msg) => {
                if auth_msg.typ() != Type::Auth {
                    return Err(anyhow!("auth failed"));
                }
                let server_info = ServerInfo::from(auth_msg.payload());
                let res_server_info = ServerInfo {
                    id: my_id(),
                    address: CONFIG.server.service_address,
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
                client_map.0.insert(server_info.id, io_channel.0.clone());
                super::handler::handler_func(io_channel, timeout_channel_receiver, &server_info)
                    .await?;
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
        let mut server = ServerTimeout::new(server_config, Duration::from_millis(3000));
        let generator: NewTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(ClientConnectionHandler::new()));
        server.run(generator).await?;
        Ok(())
    }
}
