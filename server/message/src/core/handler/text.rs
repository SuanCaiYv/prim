use crate::core::{Handler, HandlerParameters};
use crate::entity::{Msg, Type};
use crate::error;
use anyhow::anyhow;
use tonic::async_trait;

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn handle_function(
        &self,
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> crate::core::Result<Msg> {
        if Type::Text != msg.typ()
            || Type::Meme != msg.typ()
            || Type::File != msg.typ()
            || Type::Image != msg.typ()
            || Type::Audio != msg.typ()
            || Type::Video != msg.typ()
        {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        todo!()
    }
}
