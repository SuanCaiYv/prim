use std::{time::Duration, net::SocketAddr};

use ahash::AHashMap;
use lib::{Result, net::{client::{ClientReqwestShare, ClientConfigBuilder}, ReqwestHandler, ReqwestHandlerMap, ReqwestHandlerGenerator, NewReqwestConnectionHandler, ReqwestOperatorManager}};

use crate::{config::CONFIG, util::my_id, service::{handler::logic::SeqNum, server::ReqwestConnectionHandler}};

pub(crate) struct Client {
    client0: ClientReqwestShare,
}

impl Client {
    pub(crate) fn new() -> Self {
        let address = CONFIG.scheduler.address;
        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let config = config_builder.build().unwrap();
        let client0 = ClientReqwestShare::new(config, Duration::from_millis(3000), my_id());
        Self { client0 }
    }

    pub(crate) async fn build(&mut self) -> Result<()> {
        let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(1, Box::new(SeqNum {}));
        let handler_map = ReqwestHandlerMap::new(handler_map);
        let generator: ReqwestHandlerGenerator =
            Box::new(move || -> Box<dyn NewReqwestConnectionHandler> {
                Box::new(ReqwestConnectionHandler::new(handler_map.clone()))
            });

        self.client0.build(generator).await
    }

    pub(crate) async fn new_connection(&self, address: SocketAddr) -> Result<ReqwestOperatorManager> {
        self.client0.new_connection(address).await?.build().await
    }
}