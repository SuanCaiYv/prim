use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    cache::redis_ops::RedisOps,
    entity::{Msg, Type},
    error::HandlerError,
    net::{server::Handler, InnerStates, InnerStatesValue, MsgSender},
    util::{timestamp, who_we_are},
    Result,
};
use tracing::{debug, error};

use crate::{
    cache::{SEQ_NUM, USER_TOKEN},
    util::jwt::verify_token,
};
use crate::{service::ClientConnectionMap, util::my_id};

use super::is_group_msg;

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let mut redis_ops;
        // to avoid borrow check conflict.
        {
            redis_ops = inner_states
                .get_mut("generic_map")
                .unwrap()
                .as_mut_generic_parameter_map()
                .unwrap()
                .get_parameter_mut::<RedisOps>()?
                .clone();
        }
        let client_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientConnectionMap>()?;
        let sender = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MsgSender>()?;
        let token = String::from_utf8_lossy(msg.payload()).to_string();
        let key: String = redis_ops
            .get(&format!("{}{}", USER_TOKEN, msg.sender()))
            .await?;
        if let Err(e) = verify_token(&token, key.as_bytes(), msg.sender()) {
            error!("auth failed: {} {}", e, token);
            return Err(anyhow!(HandlerError::Auth(e.to_string())));
        }
        debug!("token verify succeed.");
        let mut res_msg = msg.generate_ack(my_id(), msg.timestamp());
        res_msg.set_type(Type::Auth);
        client_map.insert(msg.sender(), sender.clone());
        Ok(res_msg)
    }
}

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = (**msg).clone();
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

pub(crate) struct PreProcess {}

#[async_trait]
impl Handler for PreProcess {
    async fn run(&self, msg: &mut Arc<Msg>, states: &mut InnerStates) -> Result<Msg> {
        let client_timestamp = msg.timestamp();
        let type_value = msg.typ().value();
        if type_value >= 32 && type_value < 64
            || type_value >= 64 && type_value < 96
            || type_value >= 128 && type_value < 160
        {
            let redis_ops = states
                .get_mut("generic_map")
                .unwrap()
                .as_mut_generic_parameter_map()
                .unwrap()
                .get_parameter_mut::<RedisOps>()?;
            let seq_num;
            if is_group_msg(msg.receiver()) {
                seq_num = redis_ops
                    .atomic_increment(&format!(
                        "{}{}",
                        SEQ_NUM,
                        who_we_are(msg.receiver(), msg.receiver())
                    ))
                    .await?;
            } else {
                seq_num = redis_ops
                    .atomic_increment(&format!(
                        "{}{}",
                        SEQ_NUM,
                        who_we_are(msg.sender(), msg.receiver())
                    ))
                    .await?;
            }
            match Arc::get_mut(msg) {
                Some(msg) => {
                    msg.set_seq_num(seq_num);
                    msg.set_timestamp(timestamp())
                }
                None => {
                    return Err(anyhow!("cannot get mutable reference of msg"));
                }
            };
        }
        states.insert(
            "client_timestamp".to_owned(),
            InnerStatesValue::Num(client_timestamp),
        );
        let noop = Msg::noop();
        Ok(noop)
    }
}
