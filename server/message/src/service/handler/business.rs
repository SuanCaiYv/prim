use std::sync::Arc;

use async_trait::async_trait;
use lib::{
    entity::Msg,
    net::server::{Handler, HandlerParameters},
    Result,
};

pub(crate) struct JoinGroup;

#[async_trait]
impl Handler for JoinGroup {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}

pub(crate) struct LeaveGroup;

#[async_trait]
impl Handler for LeaveGroup {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}

pub(crate) struct AddFriend;

#[async_trait]
impl Handler for AddFriend {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}

pub(crate) struct RemoveFriend;

#[async_trait]
impl Handler for RemoveFriend {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}

pub(crate) struct SystemMessage;

#[async_trait]
impl Handler for SystemMessage {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}
