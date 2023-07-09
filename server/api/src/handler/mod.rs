use anyhow::anyhow;
use chrono::{DateTime, Local};
use lib::{
    cache::redis_ops::RedisOps,
    util::jwt::{audience_of_token, verify_token},
    Result,
};
use salvo::{writing::Json, Piece, Request, Response};

use crate::{cache::USER_TOKEN, error::HandlerError};

pub(crate) mod file;
pub(crate) mod group;
pub(crate) mod msg;
pub(crate) mod relationship;
pub(crate) mod user;

pub(crate) type HandlerResult<'a, T> = std::result::Result<ResponseResult<'a, T>, HandlerError>;

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

pub(crate) async fn verify_user(req: &mut Request, redis_ops: &mut RedisOps) -> Result<u64> {
    let token = match req.headers().get("Authorization") {
        Some(token) => token.to_str().unwrap(),
        None => return Err(anyhow!("token is required.")),
    };
    let user_id = match audience_of_token(token) {
        Ok(user_id) => user_id,
        Err(err) => return Err(anyhow!("token is invalid: {}.", err)),
    };
    let redis_key = format!("{}{}", USER_TOKEN, user_id);
    let token_key = match redis_ops.get::<String>(&redis_key).await {
        Ok(token_key) => token_key,
        Err(_err) => return Err(anyhow!("user not login.")),
    };
    let _res = match verify_token(token, token_key.as_bytes(), user_id) {
        Ok(res) => res,
        Err(err) => return Err(anyhow!("token is invalid: {}.", err)),
    };
    Ok(user_id)
}
