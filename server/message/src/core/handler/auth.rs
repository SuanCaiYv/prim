use crate::cache::TOKEN_KEY;
use anyhow::anyhow;
use jwt_simple::prelude::{
    Duration, HS256Key, MACLike, NoCustomClaims, UnixTimeStamp, VerificationOptions,
};
use std::collections::HashSet;
use std::ops::Add;
use tonic::async_trait;
use tracing::{debug, warn};

use crate::core::{Handler, HandlerParameters};

use crate::entity::{Msg, Type};
use crate::error;
use crate::util::exactly_time;

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn handle_function(
        &self,
        msg: &mut Msg,
        parameters: &mut HandlerParameters,
    ) -> crate::core::Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(error::HandlerError::NotMine));
        }
        let key: String = parameters
            .redis_ops
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
