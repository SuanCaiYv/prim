use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{entity::Msg, error::HandlerError, net::InnerStates, Result};
use lib_net_tokio::net::Handler;
use tracing::debug;

use crate::service::handler::IOTaskMsg::Direct;
use crate::service::handler::IOTaskSender;
use crate::service::{
    handler::{is_group_msg, push_group_msg},
    ClientConnectionMap,
};
use crate::util::my_id;

pub(crate) struct Text;

#[async_trait]
impl Handler for Text {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value < 32 || type_value >= 64 {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientConnectionMap>()
            .unwrap();
        let io_task_sender = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<IOTaskSender>()
            .unwrap();
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
        let client_timestamp = inner_states
            .get("client_timestamp")
            .unwrap()
            .as_num()
            .unwrap();
        Ok(msg.generate_ack(my_id(), client_timestamp))
    }
}
