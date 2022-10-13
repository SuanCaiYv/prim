use crate::core::{Handler, HandlerParameters};
use crate::entity::msg;
use anyhow::anyhow;
use tonic::async_trait;
use crate::error;

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn handle_function(
        &self,
        msg: &mut msg::Msg,
        parameters: &mut HandlerParameters,
    ) -> crate::core::Result<msg::Msg> {
        if msg::Type::Text != msg.head.typ
            || msg::Type::Meme != msg.head.typ
            || msg::Type::File != msg.head.typ
            || msg::Type::Image != msg.head.typ
            || msg::Type::Audio != msg.head.typ
            || msg::Type::Video != msg.head.typ
        {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        todo!()
    }
}
