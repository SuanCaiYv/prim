use std::time::Duration;

use crate::config::CONFIG;
use ahash::AHashMap;
use lib::{
    net::{
        server::{
            HandlerList, NewConnectionHandler, NewConnectionHandlerGenerator,
            NewServerTimeoutConnectionHandler, NewServerTimeoutConnectionHandlerGenerator,
            ServerConfigBuilder, ServerTls, Handler,
        },
        MsgIOTlsServerTimeoutWrapper, MsgIOWrapper,
    },
    Result,
};

use crate::service::handler::IOTaskSender;
use async_trait::async_trait;
use tracing::error;

use super::handler::{logic::Echo, pure_text::PureText, business::{JoinGroup, LeaveGroup, AddFriend, RemoveFriend, SystemMessage}
};

pub(crate) enum InnerValue {
    #[allow(unused)]
    Str(String),
    #[allow(unused)]
    Num(u64),
}

pub(self) struct MessageConnectionHandler {
    inner_state: AHashMap<String, InnerValue>,
    io_task_sender: IOTaskSender,
    handler_list: HandlerList<InnerValue>,
}

impl MessageConnectionHandler {
    pub(self) fn new(
        io_task_sender: IOTaskSender,
        handler_list: HandlerList<InnerValue>,
    ) -> MessageConnectionHandler {
        MessageConnectionHandler {
            inner_state: AHashMap::new(),
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
            sender,
            receiver,
            self.io_task_sender.clone(),
            &self.handler_list,
            &mut self.inner_state,
        )
        .await?;
        Ok(())
    }
}

pub(self) struct MessageTlsConnectionHandler {
    inner_state: AHashMap<String, InnerValue>,
    io_task_sender: IOTaskSender,
    handler_list: HandlerList<InnerValue>,
}

impl MessageTlsConnectionHandler {
    pub(self) fn new(
        io_task_sender: IOTaskSender,
        handler_list: HandlerList<InnerValue>,
    ) -> MessageTlsConnectionHandler {
        MessageTlsConnectionHandler {
            inner_state: AHashMap::new(),
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
            sender,
            receiver,
            self.io_task_sender.clone(),
            &self.handler_list,
            &mut self.inner_state,
        )
        .await?;
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
        let mut handler_list: Vec<Box<dyn Handler<InnerValue>>> = Vec::new();
        handler_list.push(Box::new(Echo {}));
        handler_list.push(Box::new(PureText {}));
        handler_list.push(Box::new(JoinGroup {}));
        handler_list.push(Box::new(LeaveGroup {}));
        handler_list.push(Box::new(AddFriend {}));
        handler_list.push(Box::new(RemoveFriend {}));
        handler_list.push(Box::new(SystemMessage {}));
        let handler_list = HandlerList::new(handler_list);
        // todo("timeout set")!
        let mut server = lib::net::server::Server::new(server_config);
        let io_task_sender2 = io_task_sender.clone();
        let handler_list2 = handler_list.clone();
        let generator: NewConnectionHandlerGenerator = Box::new(move || {
            Box::new(MessageConnectionHandler::new(
                io_task_sender.clone(),
                handler_list.clone(),
            ))
        });
        let generator2: NewServerTimeoutConnectionHandlerGenerator = Box::new(move || {
            Box::new(MessageTlsConnectionHandler::new(
                io_task_sender2.clone(),
                handler_list2.clone(),
            ))
        });
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
