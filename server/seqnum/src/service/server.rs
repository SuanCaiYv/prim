use std::sync::Arc;

use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    net::{server::ServerConfigBuilder, GenericParameterMap, InnerStates, InnerStatesValue},
    Result,
};
use lib_net_monoio::net::{
    server::{NewReqwestConnectionHandler, ReqwestHandlerGenerator, ServerReqwestTcp},
    ReqwestHandler, ReqwestHandlerMap,
};
use local_sync::mpsc;
use tracing::error;

use crate::config::CONFIG;

use super::{get_seqnum_map, handler::seqnum::SeqNum};

pub(crate) struct ReqwestConnectionHandler {
    states: InnerStates,
    handler_map: ReqwestHandlerMap,
}

impl ReqwestConnectionHandler {
    pub(crate) fn new(handler_map: ReqwestHandlerMap) -> ReqwestConnectionHandler {
        ReqwestConnectionHandler {
            states: AHashMap::new(),
            handler_map,
        }
    }
}

#[async_trait(?Send)]
impl NewReqwestConnectionHandler for ReqwestConnectionHandler {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::bounded::Tx<ReqwestMsg>, mpsc::bounded::Rx<ReqwestMsg>),
    ) -> Result<()> {
        let (send, mut recv) = msg_operators;
        let seqnum_map = get_seqnum_map();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(seqnum_map);

        self.states.insert(
            "generic_map".to_owned(),
            InnerStatesValue::GenericParameterMap(generic_map),
        );
        loop {
            match recv.recv().await {
                Some(mut req) => {
                    let resource_id = req.resource_id();
                    let handler = self.handler_map.get(&resource_id);
                    if handler.is_none() {
                        error!("no handler for resource_id: {}", resource_id);
                        continue;
                    }
                    let handler = handler.unwrap();
                    let resp = handler.run(&mut req, &mut self.states).await;
                    if resp.is_err() {
                        error!("handler run error: {}", resp.err().unwrap());
                        continue;
                    }
                    let mut resp = resp.unwrap();
                    resp.set_req_id(req.req_id());
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

        let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(ReqwestResourceID::Seqnum, Box::new(SeqNum::new().await));
        let handler_map: ReqwestHandlerMap = Arc::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ReqwestConnectionHandler::new(handler_map.clone()))
            });

        let mut server = ServerReqwestTcp::new(server_config);
        server.run(generator).await?;
        Ok(())
    }
}
