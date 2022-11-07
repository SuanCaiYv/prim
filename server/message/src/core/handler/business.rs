use common::entity::Msg;
use common::net::server::{Handler, HandlerParameters};
use std::sync::Arc;

pub(crate) struct Relationship;

impl Handler for Relationship {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        todo!()
    }
}

pub(crate) struct Group;

impl Handler for Group {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        todo!()
    }
}
