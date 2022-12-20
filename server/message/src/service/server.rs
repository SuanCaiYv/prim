use crate::config::CONFIG;
use lib::{
    net::{
        server::{
            IOReceiver, IOSender, NewConnectionHandler, NewConnectionHandlerGenerator,
            ServerConfigBuilder,
        },
        InnerSender,
    },
    Result,
};

use async_trait::async_trait;

pub(self) struct MessageConnectionHandler {
    io_task_sender: InnerSender,
}

impl MessageConnectionHandler {
    pub(self) fn new(io_task_sender: InnerSender) -> MessageConnectionHandler {
        MessageConnectionHandler { io_task_sender }
    }
}

#[async_trait]
impl NewConnectionHandler for MessageConnectionHandler {
    async fn handle(&mut self, io_channel: (IOSender, IOReceiver)) -> Result<()> {
        super::handler::handler_func(io_channel, self.io_task_sender.clone()).await?;
        Ok(())
    }
}

pub(crate) struct Server {}

impl Server {
    pub(crate) async fn run(io_task_sender: InnerSender) -> Result<()> {
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
            Box::new(move || Box::new(MessageConnectionHandler::new(io_task_sender.clone())));
        server.run(generator).await?;
        Ok(())
    }
}
