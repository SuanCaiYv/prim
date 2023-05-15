use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    net::{
        server::{
            InnerStates, NewReqwestConnectionHandler, NewReqwestConnectionHandlerGenerator,
            ReqwestHandler, ReqwestHandlerList, ServerConfigBuilder, ServerReqwest,
        },
        ReqwestMsgIOWrapper,
    },
    Result,
};

use crate::config::CONFIG;

use super::handler::logic::SeqNum;

pub(self) struct ReqwestConnectionHandler {
    inner_states: InnerStates,
    handler_list: ReqwestHandlerList,
}

impl ReqwestConnectionHandler {
    pub(self) fn new(handler_list: ReqwestHandlerList) -> ReqwestConnectionHandler {
        ReqwestConnectionHandler {
            inner_states: AHashMap::new(),
            handler_list,
        }
    }
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestConnectionHandler {
    async fn handle(&mut self, mut io_operators: ReqwestMsgIOWrapper) -> Result<()> {
        let (sender, receiver) = io_operators.channels();
        super::handler::handler_func(sender, receiver, &self.handler_list, &mut self.inner_states)
            .await
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

        let mut handler_list: Vec<Box<dyn ReqwestHandler>> = Vec::new();
        handler_list.push(Box::new(SeqNum {}));
        let handler_list = ReqwestHandlerList::new(handler_list);

        let generator: NewReqwestConnectionHandlerGenerator =
            Box::new(move || Box::new(ReqwestConnectionHandler::new(handler_list.clone())));

        let mut server = ServerReqwest::new(server_config);
        server.run(generator).await?;
        Ok(())
    }
}
