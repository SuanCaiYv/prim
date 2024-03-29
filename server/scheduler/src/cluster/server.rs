use std::{sync::Arc, time::Duration};

use crate::{config::config, service::get_client_caller_map};
use ahash::AHashMap;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    net::{server::ServerConfigBuilder, GenericParameterMap, InnerStates, InnerStatesValue},
    Result,
};

use async_trait::async_trait;
use lib_net_tokio::net::{
    server::{ReqwestCaller, ServerReqwest},
    NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator, ReqwestHandlerMap,
};
use tokio::sync::mpsc;
use tracing::error;

use super::{
    get_cluster_caller_map,
    handler::{logic, message},
};

pub(super) struct ClientConnectionHandler {
    handler_map: ReqwestHandlerMap,
    states: InnerStates,
    client_caller: Option<ReqwestCaller>,
}

impl ClientConnectionHandler {
    pub(super) fn new(handler_map: ReqwestHandlerMap) -> ClientConnectionHandler {
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
        let client_map = get_client_caller_map();
        let cluster_map = get_cluster_caller_map();
        let client_caller = self.client_caller.take().unwrap();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(client_map);
        generic_map.put_parameter(cluster_map);
        generic_map.put_parameter(client_caller);

        self.states.insert(
            "generic_map".to_string(),
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
                    get_cluster_caller_map().remove(node_id);
                    break;
                }
            }
        }
        Ok(())
    }

    fn set_reqwest_caller(&mut self, client_caller: ReqwestCaller) {
        self.client_caller = Some(client_caller);
    }
}

pub(crate) struct Server {}

impl Server {
    pub(crate) async fn run() -> Result<()> {
        let bind_port = config()
            .server
            .cluster_address
            .split(":")
            .last()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let bind_address = if config().server.ipv4 {
            if config().server.public_service {
                format!("[::]:{}", bind_port)
            } else {
                format!("[::1]:{}", bind_port)
            }
        } else {
            if config().server.public_service {
                format!("0.0.0.0:{}", bind_port)
            } else {
                format!("127.0.0.1:{}", bind_port)
            }
        };
        let mut server_config_builder = ServerConfigBuilder::default();
        server_config_builder
            .with_address(bind_address.parse().unwrap())
            .with_cert(config().server.cert.clone())
            .with_key(config().server.key.clone())
            .with_max_connections(config().server.max_connections)
            .with_connection_idle_timeout(config().transport.connection_idle_timeout)
            .with_max_bi_streams(config().transport.max_bi_streams);
        let server_config = server_config_builder.build().unwrap();

        let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(ReqwestResourceID::NodeAuth, Box::new(logic::ServerAuth {}));
        handler_map.insert(
            ReqwestResourceID::MessageNodeRegister,
            Box::new(message::NodeRegister {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeUnregister,
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
