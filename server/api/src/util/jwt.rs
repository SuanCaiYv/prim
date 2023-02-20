use anyhow::anyhow;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lib::util::timestamp;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    /// Optional. Audience
    aud: u64,
    /// Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    exp: u64,
    /// Optional. Issued at (as UTC timestamp)
    iat: u64,
    /// Optional. Issuer
    iss: String,
    /// Optional. Not Before (as UTC timestamp)
    nbf: u64,
    /// Optional. Subject (whom token refers to)
    sub: String,
}

#[allow(unused)]
#[inline]
pub(crate) fn simple_token(key: &[u8], audience: u64) -> String {
    let t = timestamp();
    encode(
        &Header::default(),
        &Claims {
            aud: audience,
            exp: t + 7 * 24 * 60 * 60 * 1000,
            iat: t,
            iss: "PRIM".to_string(),
            nbf: t,
            sub: "".to_string(),
        },
        &EncodingKey::from_secret(key),
    )
    .unwrap()
}

#[allow(unused)]
#[inline]
pub(crate) fn audience_of_token(token: &str) -> anyhow::Result<u64> {
    let payload = token.split('.').nth(1).unwrap();
    let res = base64::decode_config(payload, base64::URL_SAFE)?;
    let claim = serde_json::from_slice::<Claims>(res.as_slice())?;
    Ok(claim.aud)
}

#[allow(unused)]
#[inline]
pub(crate) fn verify_token(token: &str, key: &[u8], audience: u64) -> anyhow::Result<()> {
    let res = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(key),
        &Validation::default(),
    )?;
    if res.claims.aud != audience {
        return Err(anyhow!("invalid token"));
    }
    if res.claims.exp < timestamp() {
        return Err(anyhow!("token expired"));
    }
    if res.claims.iss != "PRIM".to_string() {
        return Err(anyhow!("invalid token"));
    }
    Ok(())
}
