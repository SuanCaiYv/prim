use std::{collections::HashSet, ops::Add};

use anyhow::anyhow;
use jwt_simple::prelude::{
    Claims, Duration, HS256Key, MACLike, NoCustomClaims, UnixTimeStamp, VerificationOptions,
};
use lib::{util::exactly_time, Result};
use tracing::warn;

#[allow(unused)]
#[inline]
pub(crate) fn simple_token(key: &[u8], audience: u64) -> String {
    let key = HS256Key::from_bytes(key);
    let mut claims = Claims::create(Duration::from_mins(120))
        .with_issuer("prim")
        .with_audience(audience.to_string());
    let time = exactly_time();
    let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
    claims.issued_at = Some(now);
    Some(UnixTimeStamp::new(time.0, (time.1 % 1000) as u32).add(Duration::from_secs(15)));
    let token = key.authenticate(claims);
    token.unwrap()
}

#[allow(unused)]
pub(crate) fn verify_token(key: &[u8], token: &str, audience: u64) -> Result<()> {
    let key = HS256Key::from_bytes(key);
    let mut options = VerificationOptions::default();
    options.allowed_issuers = Some(HashSet::from(["prim".to_string()]));
    options.allowed_audiences = Some(HashSet::from([audience.to_string()]));
    let claims = key.verify_token::<NoCustomClaims>(token, Some(options));
    if claims.is_err() {
        warn!("token verify failed: {}.", claims.err().unwrap());
        return Err(anyhow!("token verify error."));
    }
    let time = exactly_time();
    let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
    let claims = claims.unwrap();
    if claims.issued_at.unwrap().add(Duration::from_secs(5)) < now {
        return Err(anyhow!("token expired."));
    }
    if claims.expires_at.unwrap() < now {
        return Err(anyhow!("token expired."));
    }
    Ok(())
}
