use std::time::Duration;

use crate::{
    cluster::MsgSender,
    config::CONFIG,
    get_io_task_sender,
    service::{handler::IOTaskSender, server::InnerValue},
};
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

pub(self) struct ClusterConnectionHandler {
    handler_list: HandlerList<InnerValue>,
    inner_states: InnerStates<InnerValue>,
    io_task_sender: IOTaskSender,
}

impl ClusterConnectionHandler {
    pub(self) fn new(
        handler_list: HandlerList<InnerValue>,
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
impl NewTimeoutConnectionHandler for ClusterConnectionHandler {
    async fn handle(&mut self, mut io_operators: MsgIOTimeoutWrapper) -> Result<()> {
        let (sender, mut receiver, timeout) = io_operators.channels();
        super::handler::handler_func(
            MsgSender::Server(sender),
            receiver,
            timeout,
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
        let mut server = ServerTimeout::new(server_config, Duration::from_millis(3000));
        let handler_list: Vec<Box<dyn Handler<InnerValue>>> = Vec::new();
        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let generator: NewTimeoutConnectionHandlerGenerator = Box::new(move || {
            Box::new(ClusterConnectionHandler::new(
                handler_list.clone(),
                io_task_sender.clone(),
            ))
        });
        server.run(generator).await?;
        Ok(())
    }
}
