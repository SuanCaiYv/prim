// use std::time::Duration;

// use ahash::AHashMap;

// use common::scheduler::connect2scheduler;
// use lib::{
//     entity::{ReqwestResourceID, ServerInfo, ServerStatus, ServerType},
//     net::{client::ClientConfigBuilder, InnerStates},
//     Result,
// };
// use lib_net_tokio::net::{ReqwestHandler, ReqwestHandlerMap};

// use crate::{config::CONFIG, scheduler::handler::logic, util::my_id};

// pub(super) struct Client {}

// impl Client {
//     pub(super) async fn run() -> Result<()> {
//         let scheduler_address = CONFIG.scheduler.address;

//         let mut config_builder = ClientConfigBuilder::default();
//         config_builder
//             .with_remote_address(scheduler_address)
//             .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
//             .with_domain(CONFIG.scheduler.domain.clone())
//             .with_cert(CONFIG.scheduler.cert.clone())
//             .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
//             .with_max_bi_streams(CONFIG.transport.max_bi_streams);
//         let client_config = config_builder.build().unwrap();

//         let mut handler_map: AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>> = AHashMap::new();
//         handler_map.insert(
//             ReqwestResourceID::SeqnumNodeRegister,
//             Box::new(logic::NodeRegister {}),
//         );
//         handler_map.insert(
//             ReqwestResourceID::SeqnumNodeUnregister,
//             Box::new(logic::NodeUnregister {}),
//         );
//         let handler_map = ReqwestHandlerMap::new(handler_map);

//         let mut service_address = CONFIG.server.service_address;
//         service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
//         let server_info = ServerInfo {
//             id: my_id(),
//             service_address,
//             cluster_address: Some(service_address),
//             connection_id: 0,
//             status: ServerStatus::Online,
//             typ: ServerType::SeqnumCluster,
//             load: None,
//         };
//         let states_gen = Box::new(|| InnerStates::new());
//         let operator = connect2scheduler(
//             client_config,
//             Duration::from_millis(3000),
//             handler_map,
//             server_info,
//             states_gen,
//         )
//         .await?;
//         Box::leak(Box::new(operator));
//         Ok(())
//     }
// }
