use std::time::Duration;

use crate::{config::CONFIG, get_io_task_sender};
use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    net::{
        server::{
            Handler, HandlerList, NewConnectionHandler, NewConnectionHandlerGenerator,
            NewServerTimeoutConnectionHandler, NewServerTimeoutConnectionHandlerGenerator,
            ServerConfigBuilder, ServerTls,
        },
        InnerStates, MsgIOTlsServerTimeoutWrapper, MsgIOWrapper,
    },
    Result,
};
use tracing::error;

use super::handler::{
    business::{AddFriend, JoinGroup, LeaveGroup, RemoveFriend, SystemMessage},
    logic::Echo,
    pure_text::PureText,
};
use crate::service::handler::IOTaskSender;

pub(self) struct MessageConnectionHandler {
    inner_states: InnerStates,
    io_task_sender: IOTaskSender,
    handler_list: HandlerList,
}

pub(self) struct MessageTlsConnectionHandler {
    inner_states: InnerStates,
    io_task_sender: IOTaskSender,
    handler_list: HandlerList,
}

impl MessageConnectionHandler {
    pub(self) fn new(
        io_task_sender: IOTaskSender,
        handler_list: HandlerList,
    ) -> MessageConnectionHandler {
        MessageConnectionHandler {
            inner_states: AHashMap::new(),
            io_task_sender,
            handler_list,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for MessageConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOWrapper) -> Result<()> {
        let (sender, receiver) = io_operators.channels();
        super::handler::handler_func(
            lib::net::MsgSender::Server(sender),
            receiver,
            self.io_task_sender.clone(),
            &self.handler_list,
            &mut self.inner_states,
        )
        .await?;
        Ok(())
    }
}

impl MessageTlsConnectionHandler {
    pub(self) fn new(
        io_task_sender: IOTaskSender,
        handler_list: HandlerList,
    ) -> MessageTlsConnectionHandler {
        MessageTlsConnectionHandler {
            inner_states: AHashMap::new(),
            io_task_sender,
            handler_list,
        }
    }
}

#[async_trait]
impl NewServerTimeoutConnectionHandler for MessageTlsConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTlsServerTimeoutWrapper) -> Result<()> {
        let (sender, receiver, _timeout) = io_operators.channels();
        super::handler::handler_func(
            lib::net::MsgSender::Server(sender),
            receiver,
            self.io_task_sender.clone(),
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
        let mut config_builder = ServerConfigBuilder::default();
        config_builder
            .with_address(CONFIG.server.service_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let server_config = config_builder.build().unwrap();

        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(Echo {}));
        handler_list.push(Box::new(PureText {}));
        handler_list.push(Box::new(JoinGroup {}));
        handler_list.push(Box::new(LeaveGroup {}));
        handler_list.push(Box::new(AddFriend {}));
        handler_list.push(Box::new(RemoveFriend {}));
        handler_list.push(Box::new(SystemMessage {}));
        let handler_list = HandlerList::new(handler_list);
        let handler_list_tls = handler_list.clone();

        let io_task_sender = get_io_task_sender().clone();
        let io_task_sender_tls = io_task_sender.clone();

        let generator: NewConnectionHandlerGenerator = Box::new(move || {
            Box::new(MessageConnectionHandler::new(
                io_task_sender.clone(),
                handler_list.clone(),
            ))
        });
        let generator_tls: NewServerTimeoutConnectionHandlerGenerator = Box::new(move || {
            Box::new(MessageTlsConnectionHandler::new(
                io_task_sender_tls.clone(),
                handler_list_tls.clone(),
            ))
        });

        let mut server = lib::net::server::Server::new(server_config.clone());
        let mut server_tls = ServerTls::new(
            server_config,
            Duration::from_millis(CONFIG.transport.connection_idle_timeout),
        );
        tokio::spawn(async move {
            if let Err(e) = server.run(generator).await {
                error!("message server error: {}", e);
            }
        });
        server_tls.run(generator_tls).await?;
        Ok(())
    }
}
