use std::time::Duration;

use crate::config::CONFIG;
use ahash::AHashMap;
use lib::{
    net::{
        server::{
            Handler, HandlerList, InnerStates, NewTimeoutConnectionHandler,
            NewTimeoutConnectionHandlerGenerator, ServerConfigBuilder, ServerTimeout,
        },
        MsgIOTimeoutWrapper,
    },
    Result,
};

use async_trait::async_trait;

use super::handler::{logic, message};

pub(super) struct ClientConnectionHandler {
    handler_list: HandlerList,
    inner_states: InnerStates,
}

impl ClientConnectionHandler {
    pub(self) fn new(handler_list: HandlerList) -> ClientConnectionHandler {
        ClientConnectionHandler {
            handler_list,
            inner_states: AHashMap::new(),
        }
    }
}

#[async_trait]
impl NewTimeoutConnectionHandler for ClientConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTimeoutWrapper) -> Result<()> {
        let (sender, receiver, timeout) = io_operators.channels();
        super::handler::handler_func(
            lib::net::MsgSender::Server(sender),
            receiver,
            timeout,
            &self.handler_list,
            &mut self.inner_states,
        )
        .await?;
        Ok(())
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
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let server_config = server_config_builder.build().unwrap();

        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(logic::ServerAuth {}));
        handler_list.push(Box::new(message::NodeRegister {}));
        handler_list.push(Box::new(message::NodeUnregister {}));
        let handler_list = HandlerList::new(handler_list);
        // todo("timeout set")!
        let mut server = ServerTimeout::new(server_config, Duration::from_millis(3000));
        let generator: NewTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(ClientConnectionHandler::new(handler_list.clone())));
        server.run(generator).await?;
        Ok(())
    }
}
