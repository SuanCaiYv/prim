use std::sync::Arc;

use ahash::AHashMap;
use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo, ServerStatus, ServerType},
    net::{
        client::{ClientConfigBuilder, ClientReqwest},
        server::InnerStates,
        NewReqwestConnectionHandler, ReqwestHandler, ReqwestHandlerGenerator, ReqwestHandlerMap,
    },
    Result,
};
use tokio::sync::mpsc;
use tracing::error;

use crate::{config::CONFIG, util::my_id};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let scheduler_address = CONFIG.scheduler.address;

        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(scheduler_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();

        let mut client = ClientReqwest::new(
            client_config,
            std::time::Duration::from_millis(3000),
            my_id(),
        );

        struct NoopHandler {}
        #[async_trait]
        impl ReqwestHandler for NoopHandler {
            async fn run(
                &self,
                _msg: &mut ReqwestMsg,
                _states: &mut InnerStates,
            ) -> Result<ReqwestMsg> {
                Ok(ReqwestMsg::default())
            }
        }
        struct ReqwestMessageHandler {
            handler_map: ReqwestHandlerMap,
        }

        #[async_trait]
        impl NewReqwestConnectionHandler for ReqwestMessageHandler {
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
        let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(1, Box::new(NoopHandler {}));
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ReqwestMessageHandler {
                    handler_map: handler_map.clone(),
                })
            });
        let generator = Arc::new(generator);
        let _operator = client.build(generator).await?;

        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let mut cluster_address = CONFIG.server.cluster_address;
        cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SchedulerClient,
            load: None,
        };
        Ok(())
    }
}
