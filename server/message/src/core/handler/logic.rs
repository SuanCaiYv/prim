use crate::cache::TOKEN_KEY;
use async_trait::async_trait;
use common::cache::redis_ops::RedisOps;
use common::entity::{Msg, Type};
use common::error::HandlerError;
use common::net::server::{Handler, HandlerParameters};
use common::util::{exactly_time, timestamp};
use jwt_simple::prelude::{
    Duration, HS256Key, MACLike, NoCustomClaims, UnixTimeStamp, VerificationOptions,
};
use jwt_simple::reexports::anyhow::anyhow;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::Arc;
use tracing::{debug, warn};

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let res = parameters
            .generic_parameters
            .get_parameter_mut::<RedisOps>();
        if res.is_err() {
            return Err(anyhow!("need more operation parameters."));
        }
        let redis_ops = res.unwrap();
        let key: String = redis_ops
            .get(format!("{}{}", TOKEN_KEY, msg.sender()))
            .await?;
        let key = HS256Key::from_bytes(key.as_bytes());
        let token: String = String::from_utf8_lossy(msg.payload()).into();
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from(["prim".to_string()]));
        options.allowed_audiences = Some(HashSet::from([msg.sender().to_string()]));
        let claims = key.verify_token::<NoCustomClaims>(token.as_str(), Some(options));
        if claims.is_err() {
            warn!("token verify failed: {}.", claims.err().unwrap());
            return Err(anyhow!(HandlerError::Auth(
                "token verify error.".to_string()
            )));
        }
        debug!("token verify succeed.");
        let time = exactly_time();
        let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
        let claims = claims.unwrap();
        if claims.issued_at.unwrap().add(Duration::from_secs(5)) < now {
            return Err(anyhow!(HandlerError::Auth("token expired.".to_string())));
        }
        if claims.expires_at.unwrap() < now {
            return Err(anyhow!(HandlerError::Auth("token expired.".to_string())));
        }
        Ok(Msg::empty())
    }
}

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = (*msg).clone();
            res.update_receiver(msg.receiver());
            res.update_sender(0);
            res.update_timestamp(timestamp());
            Ok(res)
        } else {
            Ok(msg.generate_ack(msg.timestamp()))
        }
    }
}
