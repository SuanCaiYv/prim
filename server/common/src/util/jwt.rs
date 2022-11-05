use crate::util::exactly_time;

use jwt_simple::prelude::{Claims, Duration, HS256Key, MACLike, UnixTimeStamp};
use std::ops::Add;

#[allow(unused)]
#[inline]
pub fn simple_token(key: &[u8], audience: u64) -> String {
    let key = HS256Key::from_bytes(key);
    let mut claims = Claims::create(Duration::from_mins(120))
        .with_issuer("prim")
        .with_audience(audience.to_string());
    let time = exactly_time();
    let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
    claims.issued_at = Some(now);
    claims.expires_at = Some(now.add(Duration::from_secs(15)));
    let token = key.authenticate(claims);
    token.unwrap()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
    }
}
