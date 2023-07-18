use std::time::Duration;

use chrono::Local;
use lib::entity::{Msg, Type};
use salvo::handler;
use serde_json::json;
use tracing::error;

use crate::{
    cache::{get_redis_ops, ADD_FRIEND},
    error::HandlerError,
    model::relationship::{UserRelationship, UserRelationshipStatus},
    rpc::get_rpc_client,
    sql::DELETE_AT,
};

use super::{verify_user, HandlerResult, ResponseResult};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AddFriendReq {
    peer_id: u64,
    remark: String,
}

#[handler]
pub(crate) async fn add_friend(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
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
    let form = match req.parse_json::<AddFriendReq>().await {
        Ok(form) => form,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "request parameter missing or invalid.".to_string(),
            ))
        }
    };
    let key = format!("{}{}-{}", ADD_FRIEND, user_id, form.peer_id);
    let _res = match redis_ops.get::<String>(&key).await {
        Ok(_res) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "already request for new friend.".to_string(),
            ))
        },
        Err(_err) => {
        }
    };
    _ = redis_ops
        .set_exp(&key, &form.remark, Duration::from_secs(60 * 60 * 24 * 7))
        .await;
    let mut msg = Msg::text(user_id, form.peer_id, 0, &form.remark);
    msg.set_type(Type::AddFriend);
    let mut rpc_client = get_rpc_client().await;
    let _res = match rpc_client.call_push_msg(&msg).await {
        Ok(res) => res,
        Err(err) => {
            error!("rpc call push msg error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ConfirmAddFriendReq {
    peer_id: u64,
    passed: bool,
}

#[handler]
pub(crate) async fn confirm_add_friend(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
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
    let form = match req.parse_json::<ConfirmAddFriendReq>().await {
        Ok(form) => form,
        Err(_err) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "request parameter missing or invalid.".to_string(),
            ))
        }
    };
    let key = format!("{}{}-{}", ADD_FRIEND, form.peer_id, user_id);
    let res = match redis_ops.get::<String>(&key).await {
        Ok(res) => res,
        Err(err) => {
            error!("redis get error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "no add friend request.".to_string(),
            ));
        }
    };
    _ = redis_ops.del(&key).await;
    let user_relationship1 = UserRelationship {
        id: 0,
        user_id: user_id as i64,
        peer_id: form.peer_id as i64,
        remark: "".to_string(),
        status: UserRelationshipStatus::Normal,
        classification: "".to_string(),
        tag_list: vec![],
        info: json!(null),
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    let user_relationship2 = UserRelationship {
        id: 0,
        user_id: form.peer_id as i64,
        peer_id: user_id as i64,
        remark: "".to_string(),
        status: UserRelationshipStatus::Normal,
        classification: "".to_string(),
        tag_list: vec![],
        info: json!(null),
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    let _res1 = match user_relationship1.insert().await {
        Ok(res) => res,
        Err(err) => {
            error!("insert user relationship error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    let _res2 = match user_relationship2.insert().await {
        Ok(res) => res,
        Err(err) => {
            error!("insert user relationship error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    let remark = res;
    let mut msg = Msg::text2(user_id, form.peer_id, 0, &remark, &form.passed.to_string());
    msg.set_type(Type::AddFriend);
    let mut rpc_client = get_rpc_client().await;
    let _res = match rpc_client.call_push_msg(&msg).await {
        Ok(res) => res,
        Err(err) => {
            error!("rpc call push msg error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct FriendListResp {
    peer_id: u64,
    remark: String,
    status: u8,
    classification: String,
    tag_list: Vec<String>,
}

#[handler]
pub(crate) async fn get_friend_list(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, Vec<FriendListResp>> {
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
    let number = match req.query::<u64>("number") {
        Some(number) => number,
        None => {
            return Err(HandlerError::RequestMismatch(
                400,
                "number is required.".to_string(),
            ))
        }
    };
    let offset = match req.query::<u64>("offset") {
        Some(offset) => offset,
        None => {
            return Err(HandlerError::RequestMismatch(
                400,
                "offset is required.".to_string(),
            ))
        }
    };
    if number > 100 {
        return Err(HandlerError::RequestMismatch(
            400,
            "number must be less than 100.".to_string(),
        ));
    }
    let res =
        match UserRelationship::get_user_id(user_id as i64, number as i64, offset as i64).await {
            Ok(res) => res,
            Err(err) => {
                error!("get user relationship error: {}", err);
                return Err(HandlerError::RequestMismatch(
                    500,
                    "no relationship.".to_string(),
                ));
            }
        };
    let mut list = vec![];
    for item in res {
        list.push(FriendListResp {
            peer_id: item.peer_id as u64,
            remark: item.remark,
            status: item.status as u8,
            classification: item.classification,
            tag_list: item.tag_list,
        });
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: list,
    })
}

/// only both sides all invoke this method, the relationship will be dropped.
#[handler]
pub(crate) async fn delete_friend(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_) => {
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
                "peer_id is required.".to_string(),
            ))
        }
    };
    let res1 = match UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await {
        Ok(res) => res,
        Err(e) => {
            error!("get user relationship error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "relationship already unlocked.".to_string(),
            ));
        }
    };
    let res2 = match UserRelationship::get_user_id_peer_id(peer_id as i64, user_id as i64).await {
        Ok(res) => res,
        Err(e) => {
            error!("get user relationship error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "relationship already unlocked.".to_string(),
            ));
        }
    };
    _ = res1.delete().await;
    _ = res2.delete().await;
    let mut msg = Msg::text(user_id, peer_id, 0, "we have broken up.");
    msg.set_type(Type::RemoveFriend);
    let mut rpc_client = get_rpc_client().await;
    let _res = match rpc_client.call_push_msg(&msg).await {
        Ok(res) => res,
        Err(err) => {
            error!("rpc call push msg error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

/// 1. not friend
/// 2. friend but with different status, such as: normal, best friend, block, lover...
#[handler]
pub(crate) async fn get_peer_relationship(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, u8> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_) => {
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
                "peer_id is required.".to_string(),
            ))
        }
    };
    let res = match UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await {
        Ok(res) => res,
        Err(e) => {
            error!("get user relationship error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "not friend.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: res.status as u8,
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct UpdateRelationshipReq {
    peer_id: u64,
    remark: Option<String>,
    status: Option<u8>,
    classification: Option<String>,
    tag_list: Option<Vec<String>>,
}

/// only work on user and peer is already friend.
#[handler]
pub(crate) async fn update_relationship(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let req = match req.parse_json::<UpdateRelationshipReq>().await {
        Ok(req) => req,
        Err(e) => {
            error!("parse json error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "parameter missing or invalid.".to_string(),
            ));
        }
    };
    let mut res1 =
        match UserRelationship::get_user_id_peer_id(user_id as i64, req.peer_id as i64).await {
            Ok(res) => res,
            Err(e) => {
                error!("get user relationship error: {}", e);
                return Err(HandlerError::RequestMismatch(
                    400,
                    "not friend.".to_string(),
                ));
            }
        };
    let mut res2 =
        match UserRelationship::get_user_id_peer_id(req.peer_id as i64, user_id as i64).await {
            Ok(res) => res,
            Err(e) => {
                error!("get user relationship error: {}", e);
                return Err(HandlerError::RequestMismatch(
                    400,
                    "not friend.".to_string(),
                ));
            }
        };
    if req.remark.is_some() {
        res1.remark = req.remark.unwrap();
    }
    if req.status.is_some() {
        res1.status = UserRelationshipStatus::from(req.status.unwrap());
        res2.status = UserRelationshipStatus::from(req.status.unwrap());
    }
    if req.classification.is_some() {
        res1.classification = req.classification.unwrap();
    }
    if req.tag_list.is_some() {
        res1.tag_list = req.tag_list.unwrap();
    }
    let _res1 = match res1.update().await {
        Ok(res) => res,
        Err(e) => {
            error!("update user relationship error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "internal server error.".to_string(),
            ));
        }
    };
    let _res2 = match res2.update().await {
        Ok(res) => res,
        Err(e) => {
            error!("update user relationship error: {}", e);
            return Err(HandlerError::RequestMismatch(
                400,
                "internal server error.".to_string(),
            ));
        }
    };
    let mut msg = Msg::text(user_id, req.peer_id, 0, "relationship updated");
    msg.set_type(Type::SetRelationship);
    let mut rpc_client = get_rpc_client().await;
    let _res = match rpc_client.call_push_msg(&msg).await {
        Ok(res) => res,
        Err(err) => {
            error!("rpc call push msg error: {}", err);
            return Err(HandlerError::RequestMismatch(
                500,
                "internal server error.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}
