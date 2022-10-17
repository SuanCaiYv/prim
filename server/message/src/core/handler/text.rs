use std::sync::Arc;
use crate::error;
use anyhow::anyhow;
use tonic::async_trait;
use common::entity::{Msg, Type};
use common::net::HandlerParameters;
use crate::core::Handler;

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(
        &self,
        msg: Arc<Msg>,
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
