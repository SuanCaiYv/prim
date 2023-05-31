use std::{sync::Arc, time::Duration};

use crate::config::CONFIG;
use ahash::AHashMap;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    net::{
        server::{ClientCaller, GenericParameterMap, ServerConfigBuilder, ServerReqwest},
        InnerStates, InnerStatesValue, NewReqwestConnectionHandler, ReqwestHandler,
        ReqwestHandlerGenerator, ReqwestHandlerMap, ReqwestOperatorManager,
    },
    Result,
};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::error;

use super::{
    get_client_connection_map, get_message_node_set, get_scheduler_node_set, get_server_info_map,
    handler::{logic, message},
};

pub(super) struct ClientConnectionHandler {
    handler_map: ReqwestHandlerMap,
    states: InnerStates,
    client_caller: Option<Arc<ReqwestOperatorManager>>,
}

impl ClientConnectionHandler {
    pub(self) fn new(handler_map: ReqwestHandlerMap) -> ClientConnectionHandler {
        ClientConnectionHandler {
            handler_map,
            states: AHashMap::new(),
            client_caller: None,
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
        let client_map = get_client_connection_map();
        let server_info_map = get_server_info_map();
        let message_node_set = get_message_node_set();
        let scheduler_node_set = get_scheduler_node_set();
        let client_caller = self.client_caller.unwrap();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(client_map);
        generic_map.put_parameter(server_info_map);
        generic_map.put_parameter(message_node_set);
        generic_map.put_parameter(scheduler_node_set);
        generic_map.put_parameter(ClientCaller(client_caller));

        self.states.insert(
            "generic_map".to_string(),
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
                    break;
                }
            }
        }
        Ok(())
    }

    fn set_client_caller(&mut self, client_caller: Arc<ReqwestOperatorManager>) {
        self.client_caller = Some(client_caller);
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
