use std::sync::Arc;

use lib::{net::server::{Handler, HandlerParameters, InnerStates}, entity::{Msg, Type}, Result, error::HandlerError};

use async_trait::async_trait;
use anyhow::anyhow;

use crate::{service::server::InnerValue, util::my_id};

pub(crate) struct ClientAuth {}

#[async_trait]
impl Handler<InnerValue> for ClientAuth {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates<InnerValue>,
    ) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let authed = inner_states.get("authed");
        if authed.is_some() && authed.unwrap().is_bool() && authed.unwrap().as_bool().unwrap() {
            return Ok(msg.generate_ack(my_id(), msg.timestamp()));
        }
        let cluster_map = parameters
            .generic_parameters
            .get_parameter_mut::<ClusterConnectionMap>()?
            .0;
        let sender = parameters
            .generic_parameters
            .get_parameter::<MsgSender>()
            .unwrap();
        let res_server_info = ServerInfo::from(msg.payload());
        cluster_map.insert(res_server_info.id, sender.clone());
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}