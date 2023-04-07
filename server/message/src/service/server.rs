use std::time::Duration;

use crate::config::CONFIG;
use lib::{
    net::{
        server::{
            NewConnectionHandler, NewConnectionHandlerGenerator, NewServerTimeoutConnectionHandler,
            NewServerTimeoutConnectionHandlerGenerator, ServerConfigBuilder, ServerTls,
        },
        MsgIOTlsServerTimeoutWrapper, MsgIOWrapper,
    },
    Result,
};

use crate::service::handler::IOTaskSender;
use async_trait::async_trait;
use tracing::error;

pub(self) struct MessageConnectionHandler {
    io_task_sender: IOTaskSender,
}

impl MessageConnectionHandler {
    pub(self) fn new(io_task_sender: IOTaskSender) -> MessageConnectionHandler {
        MessageConnectionHandler { io_task_sender }
    }
}

#[async_trait]
impl NewConnectionHandler for MessageConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOWrapper) -> Result<()> {
        let (sender, receiver) = io_operators.channels();
        super::handler::handler_func(sender, receiver, self.io_task_sender.clone()).await?;
        Ok(())
    }
}

pub(self) struct MessageTlsConnectionHandler {
    io_task_sender: IOTaskSender,
}

impl MessageTlsConnectionHandler {
    pub(self) fn new(io_task_sender: IOTaskSender) -> MessageTlsConnectionHandler {
        MessageTlsConnectionHandler { io_task_sender }
    }
}

#[async_trait]
impl NewServerTimeoutConnectionHandler for MessageTlsConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTlsServerTimeoutWrapper) -> Result<()> {
        let (sender, receiver, _timeout) = io_operators.channels();
        super::handler::handler_func(sender, receiver, self.io_task_sender.clone()).await?;
        Ok(())
    }
}

pub(crate) struct Server {}

impl Server {
    pub(crate) async fn run(io_task_sender: IOTaskSender) -> Result<()> {
        let mut server_config_builder = ServerConfigBuilder::default();
        server_config_builder
            .with_address(CONFIG.server.service_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
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
        let generator2: NewServerTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(MessageTlsConnectionHandler::new(io_task_sender2.clone())));
        tokio::spawn(async move {
            if let Err(e) = server.run(generator).await {
                error!("message server error: {}", e);
            }
        });
        let mut server2 = ServerTls::new(server_config2, Duration::from_millis(3000));
        server2.run(generator2).await?;
        Ok(())
    }
}
