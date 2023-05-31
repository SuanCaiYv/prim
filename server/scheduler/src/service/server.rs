use std::{sync::Arc, time::Duration};

use crate::config::CONFIG;
use ahash::AHashMap;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    net::{
        server::{GenericParameterMap, ReqwestCaller, ServerConfigBuilder, ServerReqwest},
        InnerStates, InnerStatesValue, NewReqwestConnectionHandler, ReqwestHandler,
        ReqwestHandlerGenerator, ReqwestHandlerMap,
    },
    Result,
};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::error;

use super::{
    get_client_caller_map, get_message_node_set, get_scheduler_node_set, get_seqnum_node_set,
    get_server_info_map,
    handler::{logic, message},
};

pub(super) struct ClientConnectionHandler {
    handler_map: ReqwestHandlerMap,
    states: InnerStates,
    reqwest_caller: Option<ReqwestCaller>,
}

impl ClientConnectionHandler {
    pub(self) fn new(handler_map: ReqwestHandlerMap) -> ClientConnectionHandler {
        ClientConnectionHandler {
            handler_map,
            states: AHashMap::new(),
            reqwest_caller: None,
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
        let client_map = get_client_caller_map();
        let server_info_map = get_server_info_map();
        let message_node_set = get_message_node_set();
        let scheduler_node_set = get_scheduler_node_set();
        let seqnum_node_set = get_seqnum_node_set();
        let client_caller = self.reqwest_caller.take().unwrap();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(client_map);
        generic_map.put_parameter(server_info_map);
        generic_map.put_parameter(message_node_set);
        generic_map.put_parameter(scheduler_node_set);
        generic_map.put_parameter(client_caller);
        generic_map.put_parameter(seqnum_node_set);

        self.states.insert(
            "generic_map".to_owned(),
            InnerStatesValue::GenericParameterMap(generic_map),
        );
        loop {
            match recv.recv().await {
                Some(mut req) => {
                    let resource_id = req.resource_id();
                    match self.handler_map.get(&resource_id) {
                        Some(handler) => match handler.run(&mut req, &mut self.states).await {
                            Ok(mut resp) => {
                                resp.set_req_id(req.req_id());
                                let _ = send.send(resp).await;
                            }
                            Err(e) => {
                                error!("handler run error: {}", e);
                                continue;
                            }
                        },
                        None => {
                            error!("no handler for resource_id: {}", resource_id);
                            continue;
                        }
                    };
                }
                None => {
                    let node_id = self.states.get("node_id").unwrap().as_num().unwrap() as u32;
                    get_client_caller_map().remove(node_id);
                    get_server_info_map().remove(node_id);
                    get_message_node_set().remove(node_id);
                    get_scheduler_node_set().remove(node_id);
                    get_seqnum_node_set().remove(node_id);
                    break;
                }
            }
        }
        Ok(())
    }

    fn set_reqwest_caller(&mut self, client_caller: ReqwestCaller) {
        self.reqwest_caller = Some(client_caller);
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

        let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(
            ReqwestResourceID::NodeAuth.value(),
            Box::new(logic::ServerAuth {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeRegister.value(),
            Box::new(message::NodeRegister {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeUnregister.value(),
            Box::new(message::NodeUnregister {}),
        );
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ClientConnectionHandler::new(handler_map.clone()))
            });

        let mut server = ServerReqwest::new(server_config, Duration::from_millis(3000));
        let generator = Arc::new(generator);
        server.run(generator).await?;
        Ok(())
    }
}
