use crate::cache::redis_ops::RedisOps;
use crate::cache::TOKEN_KEY;
use crate::core::Handler;
use anyhow::anyhow;
use common::entity::{Msg, Type};
use common::net::HandlerParameters;
use common::util::exactly_time;
use jwt_simple::prelude::{
    Duration, HS256Key, MACLike, NoCustomClaims, UnixTimeStamp, VerificationOptions,
};
use std::collections::HashSet;
use std::ops::Add;
use std::sync::Arc;
use tonic::async_trait;
use tracing::{debug, warn};

use crate::error;

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
    ) -> crate::core::Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(error::HandlerError::NotMine));
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
            return Err(anyhow!(error::HandlerError::Auth(
                "token verify error.".to_string()
            )));
        }
        debug!("token verify succeed.");
        let time = exactly_time();
        let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
        let claims = claims.unwrap();
        if claims.issued_at.unwrap().add(Duration::from_secs(5)) < now {
            return Err(anyhow!(error::HandlerError::Auth(
                "token expired.".to_string()
            )));
        }
        if claims.expires_at.unwrap() < now {
            return Err(anyhow!(error::HandlerError::Auth(
                "token expired.".to_string()
            )));
        }
        Ok(Msg::empty())
    }
}
