use std::time::Duration;

use ahash::AHashMap;
use common::scheduler::connect2scheduler;
use lib::{
    entity::{ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::{client::ClientConfigBuilder, GenericParameterMap, InnerStates, InnerStatesValue},
    Result,
};
use lib_net_tokio::net::{ReqwestHandler, ReqwestHandlerMap};

use crate::{cache::get_redis_ops, config::CONFIG, util::my_id};

use super::handler::internal::{self};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let address = CONFIG.scheduler.address;
        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(address)
            .with_ipv4_type(CONFIG.scheduler.address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();
        let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(
            ReqwestResourceID::AssignMQProcessor,
            Box::new(internal::AssignProcessor {}),
        );
        let handler_map = ReqwestHandlerMap::new(handler_map);

        let service_address = CONFIG.scheduler.address;
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(service_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MsgprocessorCluster,
            load: None,
        };
        let redis_ops = get_redis_ops().await;
        let states_gen = Box::new(move || {
            let mut generic_map = GenericParameterMap(AHashMap::new());
            generic_map.put_parameter(redis_ops.clone());
            let mut states = InnerStates::new();
            states.insert(
                "generic_map".to_owned(),
                InnerStatesValue::GenericParameterMap(generic_map),
            );
            states
        });
        let _operator = connect2scheduler(
            client_config,
            Duration::from_millis(3000),
            handler_map,
            server_info,
            states_gen,
            ReqwestResourceID::MsgprocessorNodeRegister,
        )
        .await?;
        Ok(())
    }
}
