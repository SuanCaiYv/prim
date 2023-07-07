use std::time::Duration;

use chrono::Local;
use lib::entity::{Msg, Type, GROUP_ID_THRESHOLD};
use salvo::{handler, Request, Response};
use serde_json::json;
use tracing::error;

use crate::{
    cache::{get_redis_ops, CHECK_CODE, JOIN_GROUP},
    error::HandlerError,
    model::{
        group::{Group, GroupStatus},
        relationship::{UserRelationship, UserRelationshipStatus},
        user::User,
    },
    rpc::get_rpc_client,
    sql::DELETE_AT,
};

use super::{verify_user, HandlerResult, ResponseResult};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JoinGroupReq {
    group_id: u64,
    check_code: String,
}

/// invoked by someone who wants to join a group
#[handler]
pub(crate) async fn join_group(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => return Err(HandlerError::RequestMismatch(401, e.to_string())),
    };
    let form = match req.parse_json::<JoinGroupReq>().await {
        Ok(form) => form,
        Err(e) => return Err(HandlerError::ParameterMismatch(e.to_string())),
    };
    let check_code = match redis_ops
        .get::<String>(&format!("{}{}", CHECK_CODE, form.group_id))
        .await
    {
        Ok(check_code) => check_code,
        Err(e) => {
            error!("redis error: {}", e);
            return Err(HandlerError::InternalError(e.to_string()));
        }
    };
    if check_code != form.check_code {
        return Err(HandlerError::RequestMismatch(
            401,
            "check code mismatch".to_string(),
        ));
    }
    let join_group_key = format!("{}{}-{}", JOIN_GROUP, user_id, form.group_id);
    match redis_ops.get::<String>(&join_group_key).await {
        Ok(_) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "you have already applied for joining this group.".to_string(),
            ));
        }
        Err(_) => {
            if let Err(e) = redis_ops
                .set_exp(
                    &join_group_key,
                    &form.check_code,
                    Duration::from_secs(3 * 24 * 60 * 60),
                )
                .await
            {
                error!("redis set_exp error: {}", e.to_string());
                return Err(HandlerError::InternalError("".to_string()));
            }
            let mut rpc_client = get_rpc_client().await;
            let group = match Group::get_group_id(form.group_id as i64).await {
                Ok(group) => group,
                Err(_) => {
                    return Err(HandlerError::RequestMismatch(
                        406,
                        "group not found.".to_string(),
                    ))
                }
            };
            let admin_list = &group.admin_list;
            for admin in admin_list.iter() {
                let admin_user_id = admin
                    .as_object()
                    .unwrap()
                    .get("user_id")
                    .unwrap()
                    .as_u64()
                    .unwrap();
                let mut msg = Msg::raw2(
                    user_id,
                    admin_user_id,
                    0,
                    serde_json::to_vec(&form).unwrap().as_slice(),
                    user_id.to_string().as_bytes(),
                );
                msg.set_type(Type::JoinGroup);
                match rpc_client.call_push_msg(&msg).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("rpc call_push_msg error: {}", e.to_string());
                        return Err(HandlerError::InternalError("".to_string()));
                    }
                }
            }
            Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            })
        }
    }
}

/// invoked by someone who wants to leave a group
/// the leave msg will be pushed to every admin of the group
#[handler]
pub(crate) async fn leave_group(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => return Err(HandlerError::RequestMismatch(401, e.to_string())),
    };
    let group_id = match req.query::<u64>("group_id") {
        Some(group_id) => group_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "leave group parameter mismatch.".to_string(),
            ))
        }
    };
    let user_relationship =
        match UserRelationship::get_user_id_peer_id(user_id as i64, group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(_) => {
                return Err(HandlerError::RequestMismatch(
                    400,
                    "user not in this group.".to_string(),
                ))
            }
        };
    let mut group = match Group::get_group_id(group_id as i64).await {
        Ok(group) => group,
        Err(_) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "group not found.".to_string(),
            ))
        }
    };
    let admin_list = &group.admin_list;
    let mut rpc_client = get_rpc_client().await;
    for admin in admin_list.iter() {
        let admin_user_id = admin
            .as_object()
            .unwrap()
            .get("user_id")
            .unwrap()
            .as_f64()
            .unwrap() as u64;
        let mut msg = Msg::raw(user_id, admin_user_id, 0, &user_id.to_string().as_bytes());
        msg.set_type(Type::LeaveGroup);
        let _res = match rpc_client.call_push_msg(&msg).await {
            Ok(_) => (),
            Err(e) => {
                error!("rpc call_push_msg error: {}", e.to_string());
                return Err(HandlerError::InternalError(
                    "internal server error.".to_string(),
                ));
            }
        };
    }
    _ = user_relationship.delete().await;
    let user_role = user_relationship
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    if user_role == "admin" {
        let mut admin_list = group.admin_list;
        admin_list.retain(|admin| {
            admin
                .as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_f64()
                .unwrap() as u64
                != user_id
        });
        group.admin_list = admin_list;
    }
    if user_role == "member" {
        let mut member_list = group.member_list;
        member_list.retain(|member| {
            member
                .as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_f64()
                .unwrap() as u64
                != user_id
        });
        group.member_list = member_list;
    }
    let _res = match group.update().await {
        Ok(_) => (),
        Err(e) => {
            error!("group update error: {}", e.to_string());
            return Err(HandlerError::InternalError(
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateGroupReq {
    group_name: String,
    check_code: String,
}

#[handler]
pub(crate) async fn create_group(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, u64> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let form = match req.parse_json::<CreateGroupReq>().await {
        Ok(form) => form,
        Err(e) => return Err(HandlerError::RequestMismatch(400, e.to_string())),
    };
    let mut group_id;
    loop {
        group_id = fastrand::u64(GROUP_ID_THRESHOLD..1 << 46);
        let group = Group::get_group_id(group_id as i64).await;
        if group.is_err() {
            break;
        }
    }
    if let Err(e) = redis_ops
        .set(&format!("{}{}", CHECK_CODE, group_id), &form.check_code)
        .await
    {
        error!("redis set check code error: {}.", e.to_string());
        return Err(HandlerError::InternalError(
            "internal server error.".to_string(),
        ));
    }
    let group = Group {
        id: 0,
        group_id: group_id as i64,
        name: form.group_name,
        avatar: "".to_string(),
        admin_list: vec![json!({
            "user_id": user_id,
            "remark": user_id.to_string(),
        })],
        member_list: vec![],
        status: GroupStatus::Normal,
        info: json!({}),
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    if let Err(e) = group.insert().await {
        error!("insert group error: {}.", e.to_string());
        return Err(HandlerError::InternalError(
            "internal server error.".to_string(),
        ));
    }
    let user_relationship = UserRelationship {
        id: 0,
        user_id: user_id as i64,
        peer_id: group_id as i64,
        remark: group.name.clone(),
        status: UserRelationshipStatus::Normal,
        classification: "default".to_string(),
        tag_list: vec![],
        info: json!({
            "role": "admin",
        }),
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    if let Err(e) = user_relationship.insert().await {
        error!("insert user relationship error: {}.", e.to_string());
        return Err(HandlerError::InternalError(
            "internal server error.".to_string(),
        ));
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: group_id,
    })
}

/// every admin can invoke this method.
#[handler]
pub(crate) async fn destroy_group(_req: &mut Request, _resp: &mut Response) {
    todo!()
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupInfoResp {
    name: String,
    avatar: String,
    admin_number: u32,
    member_number: u32,
    status: u8,
    info: serde_json::Value,
}

/// the user list is excluded from the response.
#[handler]
pub(crate) async fn get_group_info(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, GroupInfoResp> {
    let group_id = match req.query::<u64>("group_id") {
        Some(group_id) => group_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "group id is required.".to_string(),
            ))
        }
    };
    let group = match Group::get_group_id(group_id as i64).await {
        Ok(group) => group,
        Err(e) => {
            error!("get group error: {}.", e.to_string());
            return Err(HandlerError::InternalError(
                "internal server error.".to_string(),
            ));
        }
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: GroupInfoResp {
            name: group.name,
            avatar: group.avatar,
            admin_number: group.admin_list.len() as u32,
            member_number: group.member_list.len() as u32,
            status: group.status as u8,
            info: group.info,
        },
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpdateGroupInfoReq {
    group_id: u64,
    name: Option<String>,
    avatar: Option<String>,
    info: Option<serde_json::Value>,
}

#[handler]
pub(crate) async fn update_group_info(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ))
        }
    };
    let form = match req.parse_json::<UpdateGroupInfoReq>().await {
        Ok(form) => form,
        Err(e) => return Err(HandlerError::RequestMismatch(400, e.to_string())),
    };
    let mut group = match Group::get_group_id(form.group_id as i64).await {
        Ok(group) => group,
        Err(e) => {
            error!("get group error: {}.", e.to_string());
            return Err(HandlerError::InternalError(
                "internal server error.".to_string(),
            ));
        }
    };
    let user_relationship =
        match UserRelationship::get_user_id_peer_id(user_id as i64, form.group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(e) => {
                error!("get user group list error: {}.", e.to_string());
                return Err(HandlerError::InternalError(
                    "internal server error.".to_string(),
                ));
            }
        };
    let user_role = user_relationship
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    if user_role != "admin" {
        return Err(HandlerError::RequestMismatch(
            403,
            "you are not admin of this group.".to_string(),
        ));
    }
    if form.name.is_some() {
        group.name = form.name.unwrap();
    }
    if form.avatar.is_some() {
        group.avatar = form.avatar.unwrap();
    }
    if form.info.is_some() {
        let info = form.info.unwrap();
        let info_map = info.as_object().unwrap();
        let group_info_map = group.info.as_object_mut().unwrap();
        for (k, v) in info_map {
            group_info_map.insert(k.to_string(), v.clone());
        }
    }
    if let Err(e) = group.update().await {
        error!("update group error: {}.", e.to_string());
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
pub(crate) async fn get_group_user_list(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, Vec<serde_json::Value>> {
    let group_id = match req.query::<u64>("group_id") {
        Some(group_id) => group_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "group id is required.".to_string(),
            ))
        }
    };
    let user_role = match req.query::<String>("user_role") {
        Some(user_role) => user_role,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "user role is required.".to_string(),
            ))
        }
    };
    let offset = match req.query::<u32>("offset") {
        Some(offset) => offset,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "offset is required.".to_string(),
            ))
        }
    };
    let limit = match req.query::<u32>("limit") {
        Some(limit) => limit,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "limit is required.".to_string(),
            ))
        }
    };
    let group = match Group::get_group_id(group_id as i64).await {
        Ok(group) => group,
        Err(e) => {
            error!("get group error: {}.", e.to_string());
            return Err(HandlerError::InternalError("group not found.".to_string()));
        }
    };
    match user_role.as_str() {
        "admin" => {
            let offset = if offset as usize > group.admin_list.len() {
                group.admin_list.len()
            } else {
                offset as usize
            };
            let limit = if limit as usize + offset > group.admin_list.len() {
                group.admin_list.len()
            } else {
                limit as usize + offset
            };
            Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: group.admin_list.as_slice()[offset..limit].to_vec(),
            })
        }
        "member" => {
            let offset = if offset as usize > group.member_list.len() {
                group.member_list.len()
            } else {
                offset as usize
            };
            let limit = if limit as usize + offset > group.member_list.len() {
                group.member_list.len()
            } else {
                limit as usize + offset
            };
            Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: group.member_list.as_slice()[offset..limit].to_vec(),
            })
        }
        _ => Ok(ResponseResult {
            code: 400,
            message: "user role is invalid.",
            timestamp: Local::now(),
            data: vec![],
        }),
    }
}

/// invoked for admin user to kick some unfortunate man out of the group.
#[handler]
pub(crate) async fn remove_member(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => {
            error!("verify user error: {}.", e.to_string());
            return Err(HandlerError::InternalError(
                "internal server error.".to_string(),
            ));
        }
    };
    let peer_id = match req.query::<u64>("user_id") {
        Some(peer_id) => peer_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let group_id = match req.query::<u64>("group_id") {
        Some(group_id) => group_id,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "group id is required.".to_string(),
            ))
        }
    };
    let reason = match req.query("reason") {
        Some(reason) => reason,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "reason is required.".to_string(),
            ))
        }
    };
    let user_relationship =
        match UserRelationship::get_user_id_peer_id(user_id as i64, group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(e) => {
                error!("get user relationship error: {}.", e.to_string());
                return Err(HandlerError::InternalError(
                    "operation user id mismatch.".to_string(),
                ));
            }
        };
    let user_role = user_relationship
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    if user_role != "admin" {
        return Err(HandlerError::RequestMismatch(
            400,
            "operation user is not admin.".to_string(),
        ));
    }
    let mut group = match Group::get_group_id(group_id as i64).await {
        Ok(group) => group,
        Err(e) => {
            error!("get group error: {}.", e.to_string());
            return Err(HandlerError::InternalError(
                "operation group id mismatch.".to_string(),
            ));
        }
    };
    let peer_group_list =
        match UserRelationship::get_user_id_peer_id(peer_id as i64, group_id as i64).await {
            Ok(peer_group_list) => peer_group_list,
            Err(e) => {
                error!("get peer group list error: {}.", e.to_string());
                return Err(HandlerError::InternalError(
                    "operation peer id mismatch.".to_string(),
                ));
            }
        };
    let peer_role = peer_group_list
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    // not allow to remove admin.
    if peer_role == "admin" {
        return Err(HandlerError::RequestMismatch(
            400,
            "peer user is admin.".to_string(),
        ));
    }
    let member_list = &mut group.member_list;
    member_list.retain(|x| {
        x.as_object()
            .unwrap()
            .get("user_id")
            .unwrap()
            .as_f64()
            .unwrap() as u64
            != peer_id
    });
    _ = group.update().await;
    _ = peer_group_list.delete().await;
    let mut msg = Msg::raw(
        user_id,
        peer_id,
        0,
        serde_json::to_vec(&json!({
            "user_id": peer_id,
            "reason": reason,
        }))
        .unwrap()
        .as_slice(),
    );
    msg.set_type(Type::LeaveGroup);
    let mut rpc_client = get_rpc_client().await;
    let _res = match rpc_client.call_push_msg(&msg).await {
        Ok(res) => res,
        Err(e) => {
            error!("call push msg error: {}.", e.to_string());
            return Err(HandlerError::InternalError(
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ApproveJoinReq {
    group_id: u64,
    peer_id: u64,
    approved: bool,
}

/// invoked for admin user to approve someone to join the group.
#[handler]
pub(crate) async fn approve_join(
    req: &mut Request,
    _resp: &mut Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => {
            error!("verify user error: {}.", e.to_string());
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized.".to_string(),
            ));
        }
    };
    let form = match req.parse_json::<ApproveJoinReq>().await {
        Ok(form) => form,
        Err(e) => {
            error!("parse json error: {}.", e.to_string());
            return Err(HandlerError::ParameterMismatch(
                "approve join parameter mismatch.".to_string(),
            ));
        }
    };
    // todo notify every member in the group.
    if form.approved {
        let group = match Group::get_group_id(form.group_id as i64).await {
            Ok(group) => group,
            Err(e) => {
                error!("get group error: {}.", e.to_string());
                return Err(HandlerError::InternalError(
                    "operation group id mismatch.".to_string(),
                ));
            }
        };
        let user_relationship = UserRelationship {
            id: 0,
            user_id: form.peer_id as i64,
            peer_id: form.group_id as i64,
            remark: group.name.clone(),
            status: UserRelationshipStatus::Normal,
            classification: "default".to_string(),
            tag_list: vec![],
            info: json!({
                "role": "member",
            }),
            create_at: Local::now(),
            update_at: Local::now(),
            delete_at: DELETE_AT.clone(),
        };
        // todo check if user already in the group.
        let _res = match user_relationship.insert().await {
            Ok(res) => {
                let user = match User::get_account_id(form.peer_id as i64).await {
                    Ok(user) => user,
                    Err(e) => {
                        error!("get user error: {}.", e.to_string());
                        return Err(HandlerError::InternalError(
                            "account not found.".to_string(),
                        ));
                    }
                };
                let mut group = Group::get_group_id(form.group_id as i64).await.unwrap();
                let mut member_list = group.member_list;
                member_list.push(json!({
                    "user_id": form.peer_id,
                    "remark": user.nickname,
                }));
                group.member_list = member_list;
                _ = group.update().await;
                res
            }
            Err(e) => {
                error!("insert user relationship error: {}.", e.to_string());
                return Err(HandlerError::InternalError(
                    "user may already in the group.".to_string(),
                ));
            }
        };
        let mut msg = Msg::raw(
            user_id,
            form.peer_id as u64,
            0,
            serde_json::to_vec(&json!({
                "group_id": form.group_id,
                "approved": true,
            }))
            .unwrap()
            .as_slice(),
        );
        msg.set_type(Type::JoinGroup);
        let mut rpc_client = get_rpc_client().await;
        if let Err(e) = rpc_client.call_push_msg(&msg).await {
            error!("push msg error: {:?}", e);
            return Err(HandlerError::InternalError(
                "internal server error.".to_string(),
            ));
        }
    } else {
        let mut msg = Msg::raw(
            user_id,
            form.peer_id as u64,
            0,
            serde_json::to_vec(&json!({
                "group_id": form.group_id,
                "approved": false,
            }))
            .unwrap()
            .as_slice(),
        );
        msg.set_type(Type::JoinGroup);
        let mut rpc_client = get_rpc_client().await;
        if let Err(e) = rpc_client.call_push_msg(&msg).await {
            error!("push msg error: {:?}", e);
            return Err(HandlerError::InternalError(
                "internal server error.".to_string(),
            ));
        }
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SetAdminReq {
    group_id: u64,
    user_id: u64,
    is_admin: bool,
}

/// invoked for admin user to set/unset the admin of the group.
#[handler]
pub(crate) async fn set_admin(
    req: &mut Request,
    _resp: &mut Response,
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
    let form = match req.parse_json::<SetAdminReq>().await {
        Ok(form) => form,
        Err(_) => {
            return Err(HandlerError::ParameterMismatch(
                "set admin parameter mismatch.".to_string(),
            ))
        }
    };
    let user_relationship1 =
        match UserRelationship::get_user_id_peer_id(user_id as i64, form.group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(_) => {
                return Err(HandlerError::RequestMismatch(
                    400,
                    "not in the group.".to_string(),
                ))
            }
        };
    let user_role1 = user_relationship1
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    if user_role1 != "admin" {
        return Err(HandlerError::RequestMismatch(
            400,
            "you are not admin of the group.".to_string(),
        ));
    }
    let mut user_relationship2 = match UserRelationship::get_user_id_peer_id(
        form.user_id as i64,
        form.group_id as i64,
    )
    .await
    {
        Ok(user_relationship) => user_relationship,
        Err(_) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "iser2 not in the group.".to_string(),
            ))
        }
    };
    let mut group = match Group::get_group_id(form.group_id as i64).await {
        Ok(group) => group,
        Err(_) => {
            return Err(HandlerError::RequestMismatch(
                400,
                "group not found.".to_string(),
            ))
        }
    };
    if form.is_admin {
        user_relationship2
            .info
            .as_object_mut()
            .unwrap()
            .insert("role".to_string(), json!("admin"));
        let admin_list = &mut group.admin_list;
        let flag = admin_list
            .iter()
            .filter(|admin| {
                if (*admin).as_object().unwrap()["user_id"].as_f64().unwrap() as u64 == form.user_id
                {
                    return true;
                } else {
                    return false;
                }
            })
            .count();
        if flag == 0 {
            admin_list.push(json!({
                "user_id": form.user_id,
                "remark": form.user_id.to_string(),
            }));
        }
        let member_list = &mut group.member_list;
        member_list.retain(|x| {
            x.as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_f64()
                .unwrap() as u64
                != form.user_id
        });
    } else {
        user_relationship2
            .info
            .as_object_mut()
            .unwrap()
            .insert("role".to_string(), json!("member"));
        let member_list = &mut group.member_list;
        let flag = member_list
            .iter()
            .filter(|member| {
                if (*member).as_object().unwrap()["user_id"].as_f64().unwrap() as u64
                    == form.user_id
                {
                    return true;
                } else {
                    return false;
                }
            })
            .count();
        if flag == 0 {
            member_list.push(json!({
                "user_id": form.user_id,
                "remark": form.user_id.to_string(),
            }));
        }
        let admin_list = &mut group.admin_list;
        admin_list.retain(|x| {
            x.as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_f64()
                .unwrap() as u64
                != form.user_id
        });
    }
    if let Err(e) = user_relationship2.update().await {
        error!("update user_relationship error: {}", e);
        return Err(HandlerError::InternalError(
            "internal server error.".to_string(),
        ));
    }
    if let Err(e) = group.update().await {
        error!("update group error: {}", e);
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
