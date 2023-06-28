use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    net::{
        server::ServerConfigBuilder,
        InnerStates, InnerStatesValue, ReqwestHandler,
        ReqwestHandlerMap, GenericParameterMap,
    },
    Result,
};
use lib_net_tokio::net::{NewReqwestConnectionHandler, ReqwestHandlerGenerator, server::{ReqwestCaller, ServerReqwest}};
use tokio::sync::mpsc;
use tracing::error;

use crate::config::CONFIG;

use super::{get_client_caller_map, handler::seqnum::SeqNum, get_seqnum_map};

pub(crate) struct ReqwestConnectionHandler {
    states: InnerStates,
    handler_map: ReqwestHandlerMap,
    reqwest_caller: Option<ReqwestCaller>,
}

impl ReqwestConnectionHandler {
    pub(crate) fn new(handler_map: ReqwestHandlerMap) -> ReqwestConnectionHandler {
        ReqwestConnectionHandler {
            states: AHashMap::new(),
            handler_map,
            reqwest_caller: None,
        }
    }
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestConnectionHandler {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()> {
        let (send, mut recv) = msg_operators;
        let client_map = get_client_caller_map();
        let seqnum_map = get_seqnum_map();
        let client_caller = self.reqwest_caller.take().unwrap();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(client_map);
        generic_map.put_parameter(seqnum_map);
        generic_map.put_parameter(client_caller);

        self.states.insert(
            "generic_map".to_owned(),
            InnerStatesValue::GenericParameterMap(generic_map),
        );
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
                    let node_id = self.states.get("node_id").unwrap().as_num().unwrap() as u32;
                    get_client_caller_map().remove(node_id);
                    break;
                }
            }
        }
        Ok(())
    }

    fn set_reqwest_caller(&mut self, reqwest_caller: ReqwestCaller) {
        self.reqwest_caller = Some(reqwest_caller);
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

        let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(ReqwestResourceID::Seqnum, Box::new(SeqNum {}));
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ReqwestConnectionHandler::new(handler_map.clone()))
            });

        let mut server = ServerReqwest::new(server_config, Duration::from_millis(3000));
        let generator = Arc::new(generator);
        server.run(generator).await?;
        Ok(())
    }
}
