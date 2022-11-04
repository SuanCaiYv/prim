use anyhow::anyhow;
use async_trait::async_trait;
use common::entity::{Msg, Type};
use common::error::HandlerError;
use common::net::server::{Handler, HandlerParameters};
use std::sync::Arc;
use tracing::info;

use crate::util::my_id;

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
        if msg.receiver() == my_id() as u64 {
            let text = String::from_utf8_lossy(msg.payload()).to_string();
            info!("receive message: {}", text);
        } else {
            parameters.io_handler_sender.send(msg.clone()).await?;
        }
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
