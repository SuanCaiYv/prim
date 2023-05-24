use std::time::Duration;

use ahash::AHashMap;
use common::scheduler::connect2scheduler;
use lib::{
    entity::{ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
    net::{
        client::ClientConfigBuilder,
        server::{GenericParameterMap, Handler},
        InnerStates, ReqwestHandler, ReqwestHandlerMap, InnerStatesValue,
    },
    Result,
};

use crate::{
    cache::get_redis_ops,
    cluster::get_cluster_connection_map,
    config::CONFIG,
    get_io_task_sender,
    service::{
        get_client_connection_map,
        handler::{
            business::{AddFriend, JoinGroup, LeaveGroup, RemoveFriend, SystemMessage},
            control_text::ControlText,
        },
    },
    util::my_id,
};

use super::handler::internal::{self};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let address = &CONFIG.scheduler.address;
        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();

        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(ControlText {}));
        handler_list.push(Box::new(JoinGroup {}));
        handler_list.push(Box::new(LeaveGroup {}));
        handler_list.push(Box::new(AddFriend {}));
        handler_list.push(Box::new(RemoveFriend {}));
        handler_list.push(Box::new(SystemMessage {}));
        let mut handler_map: AHashMap<u16, Box<dyn ReqwestHandler>> = AHashMap::new();
        handler_map.insert(
            ReqwestResourceID::MessageNodeRegister.value(),
            Box::new(internal::NodeRegister {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageNodeUnregister.value(),
            Box::new(internal::NodeUnregister {}),
        );
        handler_map.insert(
            ReqwestResourceID::MessageForward,
            Box::new(internal::MessageForward { handler_list }),
        );
        let handler_map = ReqwestHandlerMap::new(handler_map);

        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(service_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SeqnumCluster,
            load: None,
        };
        let redis_ops = get_redis_ops().await;
        let states_gen = Box::new(|| {
            let mut generic_map = GenericParameterMap::new();
            generic_map.put_parameter(redis_ops.clone());
            generic_map.put_parameter(get_client_connection_map());
            generic_map.put_parameter(get_io_task_sender());
            generic_map.put_parameter(get_cluster_connection_map());
            let mut states = InnerStates::new();
            states.insert("generic_map".to_owned(), InnerStatesValue::GenericParameterMap(generic_map));
            states
        });
        let operator = connect2scheduler(
            client_config,
            my_id(),
            Duration::from_millis(3000),
            handler_map,
            server_info,
            states_gen,
        )
        .await?;
        Ok(())
    }
}
