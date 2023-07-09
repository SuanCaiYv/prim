use chrono::Local;
use hmac::{Hmac, Mac};
use lib::{
    entity::GROUP_ID_THRESHOLD,
    util::{jwt::simple_token, salt},
};
use salvo::{handler, http::ParseError, Request, Response};
use serde_json::json;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::{
    cache::{get_redis_ops, USER_TOKEN},
    error::HandlerError,
    model::{
        group::Group,
        relationship::UserRelationship,
        user::{User, UserStatus},
    },
    rpc::get_rpc_client,
    sql::DELETE_AT,
};

use super::{verify_user, HandlerResult, ResponseResult};

type HmacSha256 = Hmac<Sha256>;

#[handler]
pub(crate) async fn new_account_id(
    _: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, u64> {
    // todo optimization
    loop {
        // todo threshold range
        let id: u64 = fastrand::u64((1 << 33) + 1..GROUP_ID_THRESHOLD);
        let res = User::get_account_id(id as i64).await;
        if res.is_err() {
            break Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: id,
            });
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct LoginReq {
    account_id: u64,
    credential: String,
}

#[handler]
pub(crate) async fn login(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_ok() {
        resp.render(ResponseResult {
            code: 200,
            message: "ok.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    } else {
        info!("direct login failed: {}", user_id.err().unwrap());
    }
    let form: Result<LoginReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        error!("login failed: {}", form.err().unwrap());
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
        .set(&format!("{}{}", USER_TOKEN, form.account_id), &key)
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
    let token = simple_token(key.as_bytes(), form.account_id);
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
    account_id: u64,
    credential: String,
}

#[handler]
pub(crate) async fn signup(req: &mut Request, _resp: &mut Response) -> HandlerResult<'static, ()> {
    let form = match req.parse_json::<SignupReq>().await {
        Ok(form) => form,
        Err(_) => {
            return Err(HandlerError::ParameterMismatch(
                "signup parameters mismatch.".to_string(),
            ))
        }
    };
    let user = User::get_account_id(form.account_id as i64).await;
    if user.is_ok() {
        error!("account already signed.");
        return Err(HandlerError::RequestMismatch(
            409,
            "account already signed.".to_string(),
        ));
    } else {
        println!("{:?}", user.err().unwrap());
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
        delete_at: DELETE_AT.clone(),
    };
    let user = user.insert().await;
    if user.is_err() {
        error!("insert error: {}", user.err().unwrap());
        return Err(HandlerError::InternalError(
            "internal server error.".to_string(),
        ));
    }
    Ok(ResponseResult {
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
pub(crate) async fn which_node(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, u32> {
    let mut redis_ops = get_redis_ops().await;
    let _user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let user_id = match req.query::<u64>("user_id") {
        Some(user_id) => user_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "user id is required.".to_string(),
            ))
        }
    };
    let res = match get_rpc_client().await.call_which_node(user_id).await {
        Ok(res) => res,
        Err(err) => {
            error!("which_node error: {}", err.to_string());
            return Err(HandlerError::InternalError(err.to_string()));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: res,
    })
}

#[handler]
pub(crate) async fn which_address(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, String> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(err) => {
            warn!("verify_user error: {}", err.to_string());
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let res = match get_rpc_client().await.call_which_to_connect(user_id).await {
        Ok(res) => res,
        Err(err) => {
            error!("which_address error: {}", err.to_string());
            return Err(HandlerError::InternalError(err.to_string()));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: res,
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct UserInfoResp {
    account_id: i64,
    nickname: String,
    avatar: String,
    signature: String,
    status: u8,
    info: serde_json::Value,
}

#[handler]
pub(crate) async fn get_user_info(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, UserInfoResp> {
    let mut redis_ops = get_redis_ops().await;
    let _user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(peer_id) => peer_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let user = match User::get_account_id(peer_id as i64).await {
        Ok(user) => user,
        Err(err) => {
            error!("get_user_info error: {}", err.to_string());
            return Err(HandlerError::RequestMismatch(
                404,
                "user not found.".to_string(),
            ));
        }
    };
    let res = UserInfoResp {
        account_id: user.account_id,
        nickname: user.nickname,
        avatar: user.avatar,
        signature: user.signature,
        status: user.status as u8,
        info: user.info,
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: res,
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct UserInfoUpdateReq {
    nickname: Option<String>,
    avatar: Option<String>,
    signature: Option<String>,
    status: Option<u8>,
    info: Option<serde_json::Value>,
}

#[handler]
pub(crate) async fn update_user_info(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let req = match req.parse_json::<UserInfoUpdateReq>().await {
        Ok(req) => req,
        Err(_err) => return Err(HandlerError::ParameterMismatch("bad request.".to_string())),
    };
    let mut user = match User::get_account_id(user_id as i64).await {
        Ok(user) => user,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                404,
                "user not found.".to_string(),
            ))
        }
    };
    if req.nickname.is_some() {
        user.nickname = req.nickname.unwrap();
    }
    if req.avatar.is_some() {
        user.avatar = req.avatar.unwrap();
    }
    if req.signature.is_some() {
        user.signature = req.signature.unwrap();
    }
    if req.status.is_some() {
        user.status = UserStatus::from(req.status.unwrap());
    }
    if req.info.is_some() {
        let info = req.info.unwrap();
        let info_map = info.as_object();
        if info_map.is_some() {
            let info_map = info_map.unwrap();
            let info = user.info.as_object_mut().unwrap();
            for (k, v) in info_map.iter() {
                info.insert(k.clone(), v.clone());
            }
        }
    }
    let _res = match user.update().await {
        Ok(res) => res,
        Err(err) => {
            error!("update user info error: {}", err.to_string());
            return Err(HandlerError::InternalError(err.to_string()));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[handler]
pub(crate) async fn get_remark_avatar(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, serde_json::Value> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(peer_id) => peer_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let avatar = if peer_id >= GROUP_ID_THRESHOLD {
        let group = match Group::get_group_id(peer_id as i64).await {
            Ok(group) => group,
            Err(err) => {
                error!("get remark avatar error: {}", err.to_string());
                return Err(HandlerError::RequestMismatch(
                    404,
                    "group not found.".to_string(),
                ));
            }
        };
        group.avatar
    } else {
        let user = match User::get_account_id(peer_id as i64).await {
            Ok(user) => user,
            Err(err) => {
                error!("get remark avatar error: {}", err.to_string());
                return Err(HandlerError::RequestMismatch(
                    404,
                    "user not found.".to_string(),
                ));
            }
        };
        user.avatar
    };
    let relationship =
        match UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await {
            Ok(relationship) => relationship,
            Err(err) => {
                error!("get remark avatar error: {}", err.to_string());
                return Err(HandlerError::RequestMismatch(
                    404,
                    "relationship not found.".to_string(),
                ));
            }
        };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: json!({
            "remark": relationship.remark,
            "avatar": avatar,
        }),
    })
}

#[handler]
pub(crate) async fn get_nickname_avatar(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, serde_json::Value> {
    let peer_id = match req.query::<u64>("peer_id") {
        Some(peer_id) => peer_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let user = match User::get_account_id(peer_id as i64).await {
        Ok(user) => user,
        Err(err) => {
            error!("get nickname avatar error: {}", err.to_string());
            return Err(HandlerError::RequestMismatch(
                404,
                "user not found.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: json!({
            "nickname": user.nickname,
            "avatar": user.avatar,
        }),
    })
}
