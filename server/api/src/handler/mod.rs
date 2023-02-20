use chrono::{DateTime, Local};
use lib::cache::redis_ops::RedisOps;
use salvo::{writer::Json, Piece, Response, Request};
use anyhow::anyhow;

use crate::{util::jwt::{audience_of_token, verify_token}, cache::USER_TOKEN, error::HandlerError};

pub(crate) mod group;
pub(crate) mod msg;
pub(crate) mod relationship;
pub(crate) mod user;
pub(crate) mod file;

pub(crate) type HandlerResult = std::result::Result<(), HandlerError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ResponseResult<'a, T>
where
    T: Send + Sync + 'static,
{
    pub(crate) code: u32,
    pub(crate) message: &'a str,
    pub(crate) timestamp: DateTime<Local>,
    pub(crate) data: T,
}

impl<'a, T: Send + Sync + 'static + serde::Serialize> Piece for ResponseResult<'a, T> {
    fn render(self, res: &mut Response) {
        res.render(Json(self));
    }
}

pub(crate) async fn verify_user(req: &mut Request, redis_ops: &mut RedisOps) -> lib::Result<u64> {
    let token = req.headers().get("Authorization");
    if token.is_none() {
        return Err(anyhow!("token is required."));
    }
    let token = token.unwrap().to_str().unwrap();
    let user_id = audience_of_token(token);
    if user_id.is_err() {
        return Err(anyhow!("token is invalid."));
    }
    let user_id = user_id.unwrap();
    let redis_key = format!("{}{}", USER_TOKEN, user_id);
    let token_key = redis_ops.get::<String>(&redis_key).await;
    if token_key.is_err() {
        return Err(anyhow!("user not login."));
    }
    let token_key = token_key.unwrap();
    let res = verify_token(token, token_key.as_bytes(), user_id);
    if res.is_err() {
        return Err(anyhow!(res.err().unwrap()));
    }
    Ok(user_id)
}
