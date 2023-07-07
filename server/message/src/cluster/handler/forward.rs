use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::InnerStates,
    Result,
};
use lib_net_tokio::net::Handler;
use tracing::debug;

use crate::service::ClientConnectionMap;

pub(crate) struct Ack;

#[async_trait]
impl Handler for Ack {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        let receiver = msg.receiver();
        if msg.typ() != Type::Ack {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientConnectionMap>()
            .unwrap();
        match client_map.get(&receiver) {
            Some(client_sender) => {
                client_sender.send(msg.clone()).await?;
            }
            None => {
                debug!("receiver {} not found", receiver);
            }
        }
        Ok(Msg::noop())
    }
}
