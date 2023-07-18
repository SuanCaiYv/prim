use crate::{config::CONFIG};
use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    net::{server::ServerConfigBuilder, InnerStates},
    Result,
};
use lib_net_tokio::net::{
    server::{
        NewConnectionHandler, NewConnectionHandlerGenerator, NewConnectionHandlerGeneratorTcp,
        NewConnectionHandlerTcp,
    },
    server::{Server as UdpServer, ServerTcp},
    Handler, HandlerList, MsgIOWrapper, MsgIOWrapperTcpS, MsgSender,
};
use tracing::error;

use super::handler::{
    business::{AddFriend, JoinGroup, LeaveGroup, RemoveFriend, SystemMessage},
    logic::{Auth, Echo, PreProcess},
    pure_text::PureText,
};
use crate::service::{get_io_task_sender, handler::IOTaskSender};

pub(self) struct MessageConnectionHandler {
    inner_states: InnerStates,
    io_task_sender: IOTaskSender,
    handler_list: HandlerList,
}

pub(self) struct MessageConnectionHandlerTcp {
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
            MsgSender::Server(sender),
            receiver,
            self.io_task_sender.clone(),
            &self.handler_list,
            &mut self.inner_states,
        )
            .await?;
        Ok(())
    }
}

impl MessageConnectionHandlerTcp {
    pub(self) fn new(
        io_task_sender: IOTaskSender,
        handler_list: HandlerList,
    ) -> MessageConnectionHandlerTcp {
        MessageConnectionHandlerTcp {
            inner_states: AHashMap::new(),
            io_task_sender,
            handler_list,
        }
    }
}

#[async_trait]
impl NewConnectionHandlerTcp for MessageConnectionHandlerTcp {
    async fn handle(&mut self, mut io_operators: MsgIOWrapperTcpS) -> Result<()> {
        let (sender, receiver) = io_operators.channels();
        super::handler::handler_func(
            MsgSender::Server(sender),
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
        handler_list.push(Box::new(Auth {}));
        handler_list.push(Box::new(PreProcess::new().await));
        handler_list.push(Box::new(Echo {}));
        handler_list.push(Box::new(PureText {}));
        handler_list.push(Box::new(JoinGroup {}));
        handler_list.push(Box::new(LeaveGroup {}));
        handler_list.push(Box::new(AddFriend {}));
        handler_list.push(Box::new(RemoveFriend {}));
        handler_list.push(Box::new(SystemMessage {}));

        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let io_task_sender0 = io_task_sender.clone();
        let handler_list0 = handler_list.clone();

        let generator: NewConnectionHandlerGenerator = Box::new(move || {
            Box::new(MessageConnectionHandler::new(
                io_task_sender.clone(),
                handler_list.clone(),
            ))
        });
        let generator_tcp: NewConnectionHandlerGeneratorTcp = Box::new(move || {
            Box::new(MessageConnectionHandlerTcp::new(
                io_task_sender0.clone(),
                handler_list0.clone(),
            ))
        });

        let mut server = UdpServer::new(server_config.clone());
        let mut server_tcp = ServerTcp::new(server_config);
        tokio::spawn(async move {
            if let Err(e) = server_tcp.run(generator_tcp).await {
                error!("message server error: {}", e);
            }
        });
        server.run(generator).await?;
        Ok(())
    }
}
