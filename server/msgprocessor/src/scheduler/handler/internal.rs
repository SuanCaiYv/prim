use std::sync::Arc;

use async_trait::async_trait;
use lib_net_tokio::net::{Handler, ReqwestHandler};
use tracing::error;

use lib::{
    entity::{Msg, ReqwestMsg, ServerInfo, Type},
    error::HandlerError,
    net::InnerStates,
    Result,
};

pub(crate) struct AssignProcessor {}

#[async_trait]
impl ReqwestHandler for AssignProcessor {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct UnassignProcessor {}

#[async_trait]
impl ReqwestHandler for UnassignProcessor {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        Ok(ReqwestMsg::default())
    }
}
