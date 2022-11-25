use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};
use tracing::debug;

use crate::service::{
    handler::{is_group_msg, push_group_msg},
    ClientConnectionMap,
};

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if Type::Text != msg.typ()
            && Type::Meme != msg.typ()
            && Type::File != msg.typ()
            && Type::Image != msg.typ()
            && Type::Audio != msg.typ()
            && Type::Video != msg.typ()
        {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = &parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?
            .0;
        let receiver = msg.receiver();
        if is_group_msg(receiver) {
            push_group_msg(msg.clone(), false).await?;
        } else {
            match client_map.get(&receiver) {
                Some(client_sender) => {
                    client_sender.send(msg.clone()).await?;
                }
                None => {
                    debug!("receiver {} not found", receiver);
                }
            }
        }
        // message record has been done by first receiver, so there is no need to do it again
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
