use async_trait::async_trait;
use common::entity::Msg;
use common::net::server::{Handler, HandlerParameters};
use std::sync::Arc;

pub(crate) struct Relationship;

#[async_trait]
impl Handler for Relationship {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> common::Result<Msg> {
        todo!()
    }
}

pub(crate) struct Group;

#[async_trait]
impl Handler for Group {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> common::Result<Msg> {
        todo!()
    }
}
