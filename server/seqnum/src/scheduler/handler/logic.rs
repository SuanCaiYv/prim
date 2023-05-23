use async_trait::async_trait;
use lib::{
    entity::{ReqwestMsg, ServerInfo},
    net::{server::InnerStates, ReqwestHandler},
    Result,
};

use crate::cluster::{node_online, node_offline};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        let server_info = ServerInfo::from(msg.payload());
        node_online(server_info.cluster_address.unwrap(), server_info.id).await?;
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl ReqwestHandler for NodeUnregister {
    async fn run(&self, msg: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let server_info = ServerInfo::from(msg.payload());
        node_offline(server_info.id).await?;
        Ok(ReqwestMsg::default())
    }
}
