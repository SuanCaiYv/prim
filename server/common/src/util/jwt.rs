use crate::util::exactly_time;

use jwt_simple::prelude::{Claims, Duration, HS256Key, MACLike, UnixTimeStamp};
use std::ops::Add;

#[allow(unused)]
#[inline]
pub fn simple_token(key: String, audience: u64) -> String {
    let key = HS256Key::from_bytes(key.as_bytes());
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

#[cfg(test)]
mod tests {
    use jwt_simple::common::VerificationOptions;
    use jwt_simple::prelude::{Duration, HS256Key, MACLike, NoCustomClaims, UnixTimeStamp};
    use std::collections::HashSet;
    use std::ops::Add;

    use crate::util::exactly_time;
    use crate::util::jwt::simple_token;

    #[test]
    fn test() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NjU4NDIzNDcsImV4cCI6MTY2NTg0OTU0NywibmJmIjoxNjY1ODQyMzQ3LCJpc3MiOiJwcmltIiwiYXVkIjoiMTE1In0.nDNEY5D3fCQdGUGt7oXP5aNc-KOxkczOIf7UZ5zNa90".to_string();
        let token1 = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NjU4NDIzNDcsImV4cCI6MTY2NTg0OTU0NywibmJmIjoxNjY1ODQyMzQ3LCJpc3MiOiJwcmltIiwiYXVkIjoiOTE2In0.lFZIPBL2yyrYLXrvnF7h8YQ6J9uFtkLi8KzlF-eNa90".to_string();
        // let time = exactly_time();
        // let now = UnixTimeStamp::new(time.0, (time.1 % 1000) as u32);
        let key = HS256Key::from_bytes(b"key");
        // let mut options = VerificationOptions::default();
        // options.allowed_audiences = Some(HashSet::from([115_u64.to_string()]));
        // options.allowed_issuers = Some(HashSet::from(["prim".to_string()]));
        let claims = key.verify_token::<NoCustomClaims>(token.as_str(), None);
        println!("{:?}", claims);
        // let jwt_claims = claims.unwrap();
        // if jwt_claims.issued_at.unwrap().add(Duration::from_secs(5)) < now {
        //     panic!("token expired");
        // }
        // if jwt_claims.expires_at.unwrap() < now {
        //     panic!("token expired");
        // }
        // if !jwt_claims
        //     .audiences
        //     .unwrap()
        //     .contains(&HashSet::from([0_u64.to_string()]))
        // {
        //     panic!("token audience error");
        // }
    }
}
