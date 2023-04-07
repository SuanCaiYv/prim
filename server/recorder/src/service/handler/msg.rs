use std::sync::Arc;

use async_trait::async_trait;
use lib::{
    entity::Msg,
    net::server::{Handler, HandlerParameters, WrapMsgMpscSender},
    Result,
};
use crate::util::my_id;

pub(crate) struct Message;

#[async_trait]
impl Handler for Message {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        let buffer_sender = &parameters.generic_parameters.get_parameter::<WrapMsgMpscSender>()?.0;
        buffer_sender.send(msg.clone()).await?;
        Ok(msg.generate_ack(my_id()))
    }
}
