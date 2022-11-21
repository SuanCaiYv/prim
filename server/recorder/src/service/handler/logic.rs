use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    cache::redis_ops::RedisOps,
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    util::timestamp,
    Result,
};
use tracing::debug;

use crate::{cache::TOKEN_KEY, util::jwt::verify_token};

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let redis_ops = parameters
            .generic_parameters
            .get_parameter_mut::<RedisOps>();
        if redis_ops.is_err() {
            return Err(anyhow!("redis ops not found"));
        }
        let token = String::from_utf8_lossy(msg.payload()).to_string();
        let redis_ops = redis_ops.unwrap();
        let key: String = redis_ops
            .get(format!("{}{}", TOKEN_KEY, msg.sender()))
            .await?;
        if let Err(e) = verify_token(key.as_bytes(), &token, msg.sender()) {
            return Err(anyhow!(HandlerError::Auth(e.to_string())));
        }
        debug!("token verify succeed.");
        let mut res_msg = msg.generate_ack(msg.timestamp());
        res_msg.set_type(Type::Auth);
        Ok(res_msg)
    }
}

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> Result<Msg> {
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
            Ok(msg.generate_ack(msg.timestamp()))
        }
    }
}
