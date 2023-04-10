use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    cache::redis_ops::RedisOps,
    entity::{Msg, Type},
    error::HandlerError,
    net::{
        server::{Handler, HandlerParameters, InnerStates},
        MsgSender,
    },
    util::timestamp,
    Result,
};
use tracing::debug;

use crate::{cache::USER_TOKEN, util::jwt::verify_token};
use crate::{service::ClientConnectionMap, util::my_id};

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let mut redis_ops;
        // to avoid borrow check conflict.
        {
            redis_ops = parameters
                .generic_parameters
                .get_parameter_mut::<RedisOps>()?
                .clone();
        }
        let client_map = parameters
            .generic_parameters
            .get_parameter::<ClientConnectionMap>()?;
        let sender = parameters.generic_parameters.get_parameter::<MsgSender>()?;
        let token = String::from_utf8_lossy(msg.payload()).to_string();
        let key: String = redis_ops
            .get(&format!("{}{}", USER_TOKEN, msg.sender()))
            .await?;
        if let Err(e) = verify_token(&token, key.as_bytes(), msg.sender()) {
            return Err(anyhow!(HandlerError::Auth(e.to_string())));
        }
        debug!("token verify succeed.");
        let client_timestamp = inner_states
            .get("client_timestamp")
            .unwrap()
            .as_num()
            .unwrap();
        let mut res_msg = msg.generate_ack(my_id(), client_timestamp);
        res_msg.set_type(Type::Auth);
        client_map.insert(msg.sender(), sender.clone());
        Ok(res_msg)
    }
}

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = (*msg).clone();
            res.set_receiver(msg.receiver());
            res.set_sender(0);
            res.set_timestamp(timestamp());
            Ok(res)
        } else {
            let client_timestamp = inner_states
                .get("client_timestamp")
                .unwrap()
                .as_num()
                .unwrap();
            Ok(msg.generate_ack(my_id(), client_timestamp))
        }
    }
}
