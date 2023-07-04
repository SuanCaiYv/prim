use std::{sync::Arc, time::Duration};

use crate::{cluster::get_cluster_caller_map, config::CONFIG};
use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID, ServerInfo},
    net::{server::ServerConfigBuilder, GenericParameterMap, InnerStates, InnerStatesValue},
    Result, MESSAGE_NODE_ID_BEGINNING, SCHEDULER_NODE_ID_BEGINNING,
};
use lib_net_tokio::net::{
    server::{ReqwestCaller, ServerReqwest, ServerReqwestTcp},
    NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator, ReqwestHandlerMap,
};
use tokio::sync::mpsc;
use tracing::error;

use super::{
    get_client_caller_map, get_message_node_set, get_seqnum_node_set, get_server_info_map,
    handler::{logic, message, seqnum},
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
        let seqnum_node_set = get_seqnum_node_set();
        let cluster_map = get_cluster_caller_map();
        let client_caller = self.reqwest_caller.take().unwrap();

        let mut generic_map = GenericParameterMap(AHashMap::new());
        generic_map.put_parameter(client_map);
        generic_map.put_parameter(server_info_map);
        generic_map.put_parameter(message_node_set);
        generic_map.put_parameter(seqnum_node_set);
        generic_map.put_parameter(cluster_map);
        generic_map.put_parameter(client_caller);

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
                    if node_id >= MESSAGE_NODE_ID_BEGINNING && node_id < SCHEDULER_NODE_ID_BEGINNING
                    {
                        let mut server_info = ServerInfo::default();
                        server_info.id = node_id;
                        let mut req = ReqwestMsg::with_resource_id_payload(
                            ReqwestResourceID::MessageNodeUnregister,
                            &server_info.to_bytes(),
                        );
                        self.handler_map
                            .get(&ReqwestResourceID::MessageNodeUnregister)
                            .unwrap()
                            .run(&mut req, &mut self.states)
                            .await?;
                    } else if node_id >= SCHEDULER_NODE_ID_BEGINNING {
                        let mut server_info = ServerInfo::default();
                        server_info.id = node_id;
                        let mut req = ReqwestMsg::with_resource_id_payload(
                            ReqwestResourceID::SchedulerNodeUnregister,
                            &server_info.to_bytes(),
                        );
                        self.handler_map
                            .get(&ReqwestResourceID::SchedulerNodeUnregister)
                            .unwrap()
                            .run(&mut req, &mut self.states)
                            .await?;
                    } else {
                        let mut server_info = ServerInfo::default();
                        server_info.id = node_id;
                        let mut req = ReqwestMsg::with_resource_id_payload(
                            ReqwestResourceID::SeqnumNodeUnregister,
                            &server_info.to_bytes(),
                        );
                        self.handler_map
                            .get(&ReqwestResourceID::SeqnumNodeUnregister)
                            .unwrap()
                            .run(&mut req, &mut self.states)
                            .await?;
                    }
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

        let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(
            ReqwestResourceID::NodeAuth,
            Box::new(logic::ServerAuth {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeRegister,
            Box::new(message::NodeRegister {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeUnregister,
            Box::new(message::NodeUnregister {}),
        );
        handler_map.insert(
            ReqwestResourceID::SeqnumNodeRegister,
            Box::new(seqnum::NodeRegister {}),
        );
        handler_map.insert(
            ReqwestResourceID::SeqnumNodeUnregister,
            Box::new(seqnum::NodeUnregister {}),
        );
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ClientConnectionHandler::new(handler_map.clone()))
            });

        let mut server = ServerReqwest::new(server_config.clone(), Duration::from_millis(3000));
        let mut tcp_server = ServerReqwestTcp::new(server_config);
        let generator = Arc::new(generator);

        tokio::spawn(async )
        server.run(generator).await?;
        Ok(())
    }
}
