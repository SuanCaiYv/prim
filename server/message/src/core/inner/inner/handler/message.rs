use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use common::Result;
use common::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
};
use tracing::info;

use crate::util::my_id;

pub(super) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        if Type::Text != msg.typ()
            && Type::Meme != msg.typ()
            && Type::File != msg.typ()
            && Type::Image != msg.typ()
            && Type::Audio != msg.typ()
            && Type::Video != msg.typ()
        {
            return Err(anyhow!(HandlerError::NotMine));
        }
        if msg.receiver() == my_id() as u64 && msg.sender() != 0 {
            let text = String::from_utf8_lossy(msg.payload()).to_string();
            info!("receive message: {} from {}", text, msg.sender());
        } else {
            todo!()
        }
        Ok(Msg::no_op())
    }
}
