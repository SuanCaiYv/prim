use crate::config::CONFIG;
use lib::{
    net::{
        server::{
            IOReceiver, IOSender, NewConnectionHandler, NewConnectionHandlerGenerator, ServerTls,
            ServerConfigBuilder,
        },
        InnerSender,
    },
    Result,
};

use async_trait::async_trait;
use tracing::error;

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
    async fn handle(&mut self, io_streams: (SendStream, RecvStream)) -> Result<()> {
        super::handler::handler_func(io_streams, self.io_task_sender.clone()).await?;
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
        let mut server_config2 = server_config.clone();
        server_config2
            .address
            .set_port(server_config2.address.port() + 2);
        // todo("timeout set")!
        let mut server = lib::net::server::Server::new(server_config);
        let io_task_sender2 = io_task_sender.clone();
        let generator: NewConnectionHandlerGenerator =
            Box::new(move || Box::new(MessageConnectionHandler::new(io_task_sender.clone())));
        let generator2: NewConnectionHandlerGenerator =
            Box::new(move || Box::new(MessageConnectionHandler::new(io_task_sender2.clone())));
        tokio::spawn(async move {
            if let Err(e) = server.run(generator).await {
                error!("message server error: {}", e);
            }
        });
        let mut server2 = ServerTls::new(server_config2);
        server2.run(generator2).await?;
        Ok(())
    }
}
