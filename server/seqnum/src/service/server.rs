use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    entity::ReqwestMsg,
    net::{
        server::{ServerConfigBuilder, ServerReqwest},
        InnerStates, NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator,
        ReqwestHandlerMap,
    },
    Result,
};
use tokio::sync::mpsc;
use tracing::error;

use crate::config::CONFIG;

use super::{handler::logic::SeqNum, CLIENT_MAP};

pub(crate) struct ReqwestConnectionHandler {
    inner_states: InnerStates,
    handler_map: ReqwestHandlerMap,
}

impl ReqwestConnectionHandler {
    pub(crate) fn new(handler_map: ReqwestHandlerMap) -> ReqwestConnectionHandler {
        ReqwestConnectionHandler {
            inner_states: AHashMap::new(),
            handler_map,
        }
    }
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestConnectionHandler {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()> {
        let mut states = AHashMap::new();
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
                    let resp = handler.run(&mut msg, &mut states).await;
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
        let mut config_builder = ServerConfigBuilder::default();
        config_builder
            .with_address(CONFIG.server.service_address)
            .with_cert(CONFIG.server.cert.clone())
            .with_key(CONFIG.server.key.clone())
            .with_max_connections(CONFIG.server.max_connections)
            .with_connection_idle_timeout(CONFIG.transport.connection_idle_timeout)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let server_config = config_builder.build().unwrap();

        let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(1, Box::new(SeqNum {}));
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ReqwestConnectionHandler::new(handler_map.clone()))
            });

        let mut server = ServerReqwest::new(server_config, Duration::from_millis(3000));
        let generator = Arc::new(generator);
        let client_map = server.run(generator).await?;
        unsafe {
            CLIENT_MAP = Some(client_map);
        }
        Ok(())
    }
}
