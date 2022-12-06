use std::sync::Arc;

use async_trait::async_trait;
use lib::{
    entity::Msg,
    net::server::{Handler, HandlerParameters},
    Result,
};

pub(crate) struct Message;

#[async_trait]
impl Handler for Message {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        Ok(msg.generate_ack())
    }
}
