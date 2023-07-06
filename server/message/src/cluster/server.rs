use async_trait::async_trait;
use lib::{
    net::{server::ServerConfigBuilder, InnerStates},
    Result,
};
use lib_net_tokio::net::{
    server::{NewConnectionHandler, NewConnectionHandlerGenerator, Server as UdpServer},
    Handler, HandlerList, MsgIOWrapper,
};

use super::handler::{logic, pure_text};

use crate::{
    cluster::MsgSender, config::CONFIG, get_io_task_sender, service::handler::IOTaskSender,
};

pub(self) struct ClusterConnectionHandler {
    handler_list: HandlerList,
    inner_states: InnerStates,
    io_task_sender: IOTaskSender,
}

impl ClusterConnectionHandler {
    pub(self) fn new(
        handler_list: HandlerList,
        io_task_sender: IOTaskSender,
    ) -> ClusterConnectionHandler {
        ClusterConnectionHandler {
            handler_list,
            inner_states: InnerStates::new(),
            io_task_sender,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for ClusterConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOWrapper) -> Result<()> {
        let (sender, receiver) = io_operators.channels();
        super::handler::handler_func(
            MsgSender::Server(sender),
            receiver,
            &self.io_task_sender,
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
            .with_address(CONFIG.server.cluster_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let server_config = server_config_builder.build().unwrap();
        // todo("timeout set")!
        let mut server = UdpServer::new(server_config);
        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(logic::ServerAuth {}));
        handler_list.push(Box::new(pure_text::Text {}));
        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let generator: NewConnectionHandlerGenerator = Box::new(move || {
            Box::new(ClusterConnectionHandler::new(
                handler_list.clone(),
                io_task_sender.clone(),
            ))
        });
        server.run(generator).await?;
        Ok(())
    }
}
