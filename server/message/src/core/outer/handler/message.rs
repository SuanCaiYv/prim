use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use common::entity::{Msg, Type};
use common::error::HandlerError;
use common::net::server::{Handler, HandlerParameters};

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        if Type::Text != msg.typ()
            && Type::Meme != msg.typ()
            && Type::File != msg.typ()
            && Type::Image != msg.typ()
            && Type::Audio != msg.typ()
            && Type::Video != msg.typ()
        {
            return Err(anyhow!(HandlerError::NotMine));
        }
        // todo!("separate direct logic and forward logic");
        parameters.io_handler_sender.send(msg.clone()).await?;
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
