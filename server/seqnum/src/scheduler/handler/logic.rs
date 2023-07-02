// use async_trait::async_trait;
// use lib::{
//     entity::{ReqwestMsg, ServerInfo},
//     net::{InnerStates},
//     Result,
// };
// use lib_net_tokio::net::ReqwestHandler;

// use crate::cluster::{node_offline, node_online};

// pub(crate) struct NodeRegister {}

// #[async_trait]
// impl ReqwestHandler for NodeRegister {
//     async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
//         let new_peer = msg.payload()[0] == 1;
//         let server_info = ServerInfo::from(&(msg.payload())[1..]);
//         node_online(server_info.cluster_address.unwrap(), server_info.id, new_peer).await?;
//         Ok(ReqwestMsg::default())
//     }
// }

// pub(crate) struct NodeUnregister {}

// #[async_trait]
// impl ReqwestHandler for NodeUnregister {
//     async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
//         let server_info = ServerInfo::from(msg.payload());
//         node_offline(server_info.id).await?;
//         Ok(ReqwestMsg::default())
//     }
// }
