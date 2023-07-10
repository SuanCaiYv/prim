use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::{InnerStates, InnerStatesValue},
    Result,
};
use lib_net_tokio::net::Handler;

pub(crate) struct Ack;

#[async_trait]
impl Handler for Ack {
    async fn run(&self, msg: &mut Arc<Msg>, states: &mut InnerStates) -> Result<Msg> {
        if msg.typ() != Type::Ack {
            return Err(anyhow!(HandlerError::NotMine));
        }
        states.insert("last_ack".to_owned(), InnerStatesValue::LastAck(msg.clone()));
        Ok(Msg::noop())
    }
}
