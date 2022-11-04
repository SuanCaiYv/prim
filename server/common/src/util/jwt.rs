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
    use std::collections::HashSet;
    use jwt_simple::algorithms::HS256Key;
    use jwt_simple::claims::NoCustomClaims;
    use jwt_simple::common::VerificationOptions;
    use jwt_simple::algorithms::MACLike;
    use crate::util::jwt::simple_token;

    #[test]
    fn test() {
        let key = HS256Key::from_bytes(b"e79b3a74Vf436V46aaV8931Vc3100618");
        let t = simple_token(b"16b0d92bV92cdV44d6Vae37Vbfeabcf0", 1);
        let token: String = String::from_utf8_lossy(b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2Njc1NjcyNDIsImV4cCI6MTY2NzU2NzI1NywibmJmIjoxNjY3NTY3MjQyLCJpc3MiOiJwcmltIiwiYXVkIjoiMSJ9.h20Ci0PeHTDTFmDXPBCGzGYQZWo_2MI_tMyz8EPxJks").into();
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from(["prim".to_string()]));
        options.allowed_audiences = Some(HashSet::from(["1".to_string()]));
        let claims = key.verify_token::<NoCustomClaims>(t.as_str(), Some(options));
        println!("{:?}", claims);
    }
}
