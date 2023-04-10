use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::Msg,
    error::HandlerError,
    net::server::{Handler, HandlerParameters, InnerStates},
    Result,
};
use tracing::debug;

use crate::service::handler::IOTaskMsg::Direct;
use crate::service::{handler::IOTaskSender, server::InnerValue};
use crate::service::{
    handler::{is_group_msg, push_group_msg},
    ClientConnectionMap,
};
use crate::util::my_id;

pub(crate) struct Text;

#[async_trait]
impl Handler<InnerValue> for Text {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates<InnerValue>,
    ) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value < 32 || type_value >= 64 {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = &parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?
            .0;
        let io_task_sender = parameters
            .generic_parameters
            .get_parameter::<IOTaskSender>()?;
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
            io_task_sender.send(Direct(msg.clone())).await?;
        }
        let client_timestamp = match inner_states.get("client_timestamp").unwrap() {
            InnerValue::Num(v) => *v,
            _ => 0,
        };
        Ok(msg.generate_ack(my_id(), client_timestamp))
    }
}
