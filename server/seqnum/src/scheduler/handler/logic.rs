use std::sync::Arc;

use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    error::HandlerError,
    net::{
        server::{Handler, HandlerParameters, InnerStates},
        MsgSender,
    },
    Result,
};

use anyhow::anyhow;
use async_trait::async_trait;

use crate::{config::CONFIG, util::my_id};

pub(crate) struct ClientAuth {}

#[async_trait]
impl Handler for ClientAuth {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let authed = inner_states.get("authed");
        if authed.is_some() && authed.unwrap().is_bool() && authed.unwrap().as_bool().unwrap() {
            return Ok(msg.generate_ack(my_id(), msg.timestamp()));
        }
        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let mut cluster_address = CONFIG.server.cluster_address;
        cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
        let sender = parameters
            .generic_parameters
            .get_parameter::<MsgSender>()
            .unwrap();
        // register self to scheduler
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SeqnumCluster,
            load: None,
        };
        let mut register_msg = Msg::raw_payload(&server_info.to_bytes());
        register_msg.set_type(Type::MessageNodeRegister);
        register_msg.set_sender(server_info.id as u64);
        sender.send(Arc::new(register_msg)).await?;
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}
