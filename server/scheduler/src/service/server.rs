use std::time::Duration;

use crate::config::CONFIG;
use ahash::AHashMap;
use lib::{
    net::{
        server::{
            Handler, HandlerList, NewTimeoutConnectionHandler,
            NewTimeoutConnectionHandlerGenerator, ServerConfigBuilder, ServerTimeout, ServerReqwest,
        },
        MsgIOTimeoutWrapper, InnerStates, NewReqwestConnectionHandler, ReqwestHandlerMap,
    },
    Result, entity::ReqwestMsg,
};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::error;

use super::handler::{logic, message};

pub(super) struct ClientConnectionHandler {
    handler_map: ReqwestHandlerMap,
    states: InnerStates,
}

impl ClientConnectionHandler {
    pub(self) fn new(handler_map: ReqwestHandlerMap) -> ClientConnectionHandler {
        ClientConnectionHandler {
            handler_map,
            states: AHashMap::new(),
        }
    }
}

#[async_trait]
impl NewReqwestConnectionHandler for ClientConnectionHandler {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()> {
        let (send, mut recv) = msg_operators;
        loop {
            match recv.recv().await {
                Some(mut msg) => {
                    let resource_id = msg.resource_id();
                    let handler = self.handler_map.get(&resource_id);
                    if handler.is_none() {
                        error!("no handler for resource_id: {}", resource_id);
                        continue;
                    }
                    let handler = handler.unwrap();
                    let resp = handler.run(&mut msg, &mut self.states).await;
                    if resp.is_err() {
                        error!("handler run error: {}", resp.err().unwrap());
                        continue;
                    }
                    let resp = resp.unwrap();
                    let _ = send.send(resp).await;
                }
                None => {
                    break;
                }
            }
        }
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
        let mut server = ServerReqwest::new(server_config, Duration::from_millis(3000));
        let generator: NewTimeoutConnectionHandlerGenerator =
            Box::new(move || Box::new(ClientConnectionHandler::new(handler_list.clone())));
        let caller_map = server.run(generator).await?;
        Ok(())
    }
}
