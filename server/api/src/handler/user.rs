use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::model::user::{User, UserStatus};
use crate::rpc::get_rpc_client;
use crate::util::jwt::simple_token;
use chrono::Local;
use hmac::{Hmac, Mac};
use lib::entity::GROUP_ID_THRESHOLD;
use lib::util::salt;
use salvo::http::ParseError;
use salvo::{handler, Request, Response};
use sha2::Sha256;
use tracing::error;

use super::ResponseResult;

type HmacSha256 = Hmac<Sha256>;

#[handler]
pub(crate) async fn new_account_id(_: &mut Request, resp: &mut Response) {
    // todo optimization
    loop {
        // todo threshold range
        let id: u64 = fastrand::u64((1 << 33) + 1..GROUP_ID_THRESHOLD);
        let res = User::get_account_id(id as i64).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: id,
            });
            break;
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct LoginReq {
    account_id: f64,
    credential: String,
}

#[handler]
pub(crate) async fn login(req: &mut Request, resp: &mut Response) {
    let form: Result<LoginReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "login parameters mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let form = form.unwrap();
    let user = User::get_account_id(form.account_id as i64).await;
    if user.is_err() {
        resp.render(ResponseResult {
            code: 404,
            message: "account not found.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user = user.unwrap();
    let mut mac: HmacSha256 = HmacSha256::new_from_slice(user.salt.as_bytes()).unwrap();
    mac.update(form.credential.as_bytes());
    let res = mac.finalize().into_bytes();
    let res_str = format!("{:X}", res);
    if res_str != user.credential {
        resp.render(ResponseResult {
            code: 401,
            message: "credential mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let key = salt(12);
    let mut redis_ops = get_redis_ops().await;
    if let Err(_) = redis_ops
        .set(&format!("{}{}", TOKEN_KEY, form.account_id), &key)
        .await
    {
        error!("redis set error");
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let token = simple_token(key.as_bytes(), form.account_id as u64);
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: token,
    });
}

#[handler]
pub(crate) async fn logout(req: &mut Request, resp: &mut Response) {
    let token = req.header::<String>("Authentication");
    if token.is_none() {
        resp.render(ResponseResult {
            code: 401,
            message: "unauthorized.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    todo!("logout");
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SignupReq {
    account_id: f64,
    credential: String,
}

#[handler]
pub(crate) async fn signup(req: &mut Request, resp: &mut Response) {
    let form: Result<SignupReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "signup parameters mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let user = User::get_account_id(form.account_id as i64).await;
    if user.is_ok() {
        resp.render(ResponseResult {
            code: 409,
            message: "account already signed.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_salt = salt(12);
    let mut mac: HmacSha256 = HmacSha256::new_from_slice(user_salt.as_bytes()).unwrap();
    mac.update(form.credential.as_bytes());
    let res = mac.finalize().into_bytes();
    let res_str = format!("{:X}", res);
    let user = User {
        id: 0,
        account_id: form.account_id as i64,
        credential: res_str,
        salt: user_salt,
        nickname: form.account_id.to_string(),
        avatar: "".to_string(),
        signature: "".to_string(),
        status: UserStatus::Online,
        info: serde_json::Value::Null,
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: None,
    };
    let user = User::insert(&user).await;
    if user.is_err() {
        error!("insert error: {}", user.err().unwrap());
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[handler]
pub(crate) async fn sign_out(_req: &mut Request, _resp: &mut Response) {
    todo!("sign_out");
}

#[handler]
pub(crate) async fn which_node(req: &mut Request, resp: &mut Response) {
    let user_id = req.param::<u64>("user_id");
    if user_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "user id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_id = user_id.unwrap();
    let res = get_rpc_client().await.call_which_node(user_id).await;
    if res.is_err() {
        error!("which_node error: {}", res.err().unwrap().to_string());
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res = res.unwrap();
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: res,
    });
}

struct UserInfoResp {
    account_id: i64,
    nickname: String,
    avatar: String,
    signature: String,
    status: UserStatus,
    info: serde_json::Value,
}

#[handler]
pub(crate) async fn get_user_info(_req: &mut Request, _resp: &mut Response) {
    todo!("user_info");
}

#[handler]
pub(crate) async fn user_info_update(_req: &mut Request, _resp: &mut Response) {
    todo!("user_info_update");
}

#[handler]
pub(crate) async fn get_nickname_avatar(_req: &mut Request, _resp: &mut Response) {
    todo!("get_nickname_avatar");
}
