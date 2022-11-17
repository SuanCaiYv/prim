use std::sync::Arc;

use async_trait::async_trait;
use lib::{Result, entity::Msg, net::server::{HandlerParameters, Handler}};

pub(crate) struct Relationship;

#[async_trait]
impl Handler for Relationship {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}

pub(crate) struct Group;

#[async_trait]
impl Handler for Group {
    async fn run(&self, _msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        todo!()
    }
}
