use std::time::Duration;

use chrono::Local;
use lib::{
    entity::{Msg, Type, GROUP_ID_THRESHOLD},
    Result,
};
use salvo::{handler, Request, Response};
use serde_json::json;
use tracing::error;

use crate::error::HandlerError::{InternalError, ParameterMismatch, RequestMismatch};
use crate::model::group::GroupStatus;
use crate::sql::DELETE_AT;
use crate::{
    cache::CHECK_CODE,
    model::{
        relationship::{UserRelationship, UserRelationshipStatus},
        user::User,
    },
};
use crate::{
    cache::{get_redis_ops, JOIN_GROUP},
    model::group::Group,
    rpc::get_rpc_client,
};

use super::{verify_user, HandlerResult, ResponseResult};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JoinGroupReq {
    group_id: u64,
    check_code: String,
}

/// invoked by someone who wants to join a group
#[handler]
pub(crate) async fn join_group(req: &mut Request, resp: &mut Response) -> HandlerResult {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => return Err(RequestMismatch(401, e.to_string())),
    };
    let form = match req.parse_json::<JoinGroupReq>().await {
        Ok(form) => form,
        Err(e) => return Err(ParameterMismatch(e.to_string())),
    };
    let check_code = match redis_ops
        .get::<String>(&format!("{}{}", CHECK_CODE, form.group_id))
        .await
    {
        Ok(check_code) => check_code,
        Err(e) => {
            error!("redis error: {}", e);
            return Err(InternalError(e.to_string()));
        }
    };
    if check_code != form.check_code {
        return Err(RequestMismatch(401, "check code mismatch".to_string()));
    }
    let join_group_key = format!("{}{}-{}", JOIN_GROUP, user_id, form.group_id);
    match redis_ops.get::<String>(&join_group_key).await {
        Ok(_) => {
            resp.render(ResponseResult {
                code: 400,
                message: "you have already applied for join this group.",
                timestamp: Local::now(),
                data: (),
            });
            return Ok(());
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
                return Err(InternalError("".to_string()));
            }
            let mut rpc_client = get_rpc_client().await;
            let group = match Group::get_group_id(form.group_id as i64).await {
                Ok(group) => group,
                Err(_) => return Err(RequestMismatch(406, "group not found.".to_string())),
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
                        return Err(InternalError("".to_string()));
                    }
                }
            }
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            });
            Ok(())
        }
    }
}

/// invoked by someone who wants to leave a group
/// the leave msg will be pushed to every admin of the group
#[handler]
pub(crate) async fn leave_group(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_id = user_id.unwrap();
    let group_id: Option<u64> = req.query("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "leave group parameter mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let group_id = group_id.unwrap();
    let user_relationship =
        UserRelationship::get_user_id_peer_id(user_id as i64, group_id as i64).await;
    if user_relationship.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "user not in this group.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_relationship = user_relationship.unwrap();
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "group not found.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut group = group.unwrap();
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
        let res = rpc_client.call_push_msg(&msg).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
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
    let res = group.update().await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateGroupReq {
    group_name: String,
    check_code: String,
}

#[handler]
pub(crate) async fn create_group(req: &mut Request, resp: &mut Response) -> HandlerResult {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => return Err(RequestMismatch(401, e.to_string())),
    };
    let form = match req.parse_json::<CreateGroupReq>().await {
        Ok(form) => form,
        Err(e) => return Err(RequestMismatch(400, e.to_string())),
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
        return Err(InternalError("internal server error.".to_string()));
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
        return Err(InternalError("internal server error.".to_string()));
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
        return Err(InternalError("internal server error.".to_string()));
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: group_id.to_string(),
    });
    Ok(())
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
pub(crate) async fn get_group_info(req: &mut Request, resp: &mut Response) {
    let group_id: Option<u64> = req.query("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "group id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let group_id = group_id.unwrap();
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 404,
            message: "group not found.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let group = group.unwrap();
    resp.render(ResponseResult {
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
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpdateGroupInfoReq {
    group_id: u64,
    name: Option<String>,
    avatar: Option<String>,
    info: Option<serde_json::Value>,
}

#[handler]
pub(crate) async fn update_group_info(req: &mut Request, resp: &mut Response) -> HandlerResult {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(e) => return Err(RequestMismatch(401, e.to_string())),
    };
    let form = match req.parse_json::<UpdateGroupInfoReq>().await {
        Ok(form) => form,
        Err(e) => return Err(RequestMismatch(400, e.to_string())),
    };
    let mut group = match Group::get_group_id(form.group_id as i64).await {
        Ok(group) => group,
        Err(e) => {
            error!("get group error: {}.", e.to_string());
            return Err(InternalError("internal server error.".to_string()));
        }
    };
    let user_relationship =
        match UserRelationship::get_user_id_peer_id(user_id as i64, form.group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(e) => {
                error!("get user group list error: {}.", e.to_string());
                return Err(InternalError("internal server error.".to_string()));
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
        resp.render(ResponseResult {
            code: 403,
            message: "you are not admin of this group.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
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
        return Err(InternalError("internal server error.".to_string()));
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    });
    Ok(())
}

#[handler]
pub(crate) async fn get_group_user_list(req: &mut Request, resp: &mut Response) -> Result<()> {
    let group_id = req.query::<u64>("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "group id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
    }
    let group_id = group_id.unwrap();
    let user_role = req.query::<String>("user_role");
    if user_role.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "user role is required.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
    }
    let user_role = user_role.unwrap();
    let offset = req.query::<u32>("offset");
    if offset.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "offset is required.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
    }
    let offset = offset.unwrap();
    let limit = req.query::<u32>("limit");
    if limit.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "limit is required.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
    }
    let limit = limit.unwrap();
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 404,
            message: "group not found.",
            timestamp: Local::now(),
            data: (),
        });
        return Ok(());
    }
    let group = group.unwrap();
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
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: group.admin_list.as_slice()[offset..limit].to_vec(),
            });
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
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: group.member_list.as_slice()[offset..limit].to_vec(),
            });
        }
        _ => {
            resp.render(ResponseResult {
                code: 400,
                message: "user role is invalid.",
                timestamp: Local::now(),
                data: (),
            });
            return Ok(());
        }
    }
    Ok(())
}

/// invoked for admin user to kick some unfortunate man out of the group.
#[handler]
pub(crate) async fn remove_member(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_id = user_id.unwrap();
    let peer_id: Option<u64> = req.query("user_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let group_id: Option<u64> = req.query("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let group_id = group_id.unwrap();
    let reason: Option<&str> = req.query("reason");
    if reason.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let reason = reason.unwrap();
    let user_relationship =
        UserRelationship::get_user_id_peer_id(user_id as i64, group_id as i64).await;
    if user_relationship.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "operation user id mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_relationship = user_relationship.unwrap();
    let user_role = user_relationship
        .info
        .as_object()
        .unwrap()
        .get("role")
        .unwrap()
        .as_str()
        .unwrap();
    if user_role != "admin" {
        resp.render(ResponseResult {
            code: 400,
            message: "operation user is not admin.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "group id mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut group = group.unwrap();
    let peer_group_list =
        UserRelationship::get_user_id_peer_id(peer_id as i64, group_id as i64).await;
    if peer_group_list.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer user id mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_group_list = peer_group_list.unwrap();
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
        resp.render(ResponseResult {
            code: 400,
            message: "peer user is admin.",
            timestamp: Local::now(),
            data: (),
        });
        return;
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
    let res = rpc_client.call_push_msg(&msg).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ApproveJoinReq {
    group_id: u64,
    peer_id: u64,
    approved: bool,
}

/// invoked for admin user to approve someone to join the group.
#[handler]
pub(crate) async fn approve_join(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: "unauthorized.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form = req.parse_json::<ApproveJoinReq>().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "approve join parameter mismatch.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let form = form.unwrap();
    // todo notify every member in the group.
    if form.approved {
        let group = Group::get_group_id(form.group_id as i64).await;
        if group.is_err() {
            resp.render(ResponseResult {
                code: 400,
                message: "group id mismatch.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
        let group = group.unwrap();
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
        let res = user_relationship.insert().await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 400,
                message: "user may already in the group.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        } else {
            let user = User::get_account_id(form.peer_id as i64).await;
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
            let mut group = Group::get_group_id(form.group_id as i64).await.unwrap();
            let mut member_list = group.member_list;
            member_list.push(json!({
                "user_id": form.peer_id,
                "remark": user.nickname,
            }));
            group.member_list = member_list;
            _ = group.update().await;
        }
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
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: (),
            });
            return;
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
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SetAdminReq {
    group_id: u64,
    user_id: u64,
    is_admin: bool,
}

/// invoked for admin user to set/unset the admin of the group.
#[handler]
pub(crate) async fn set_admin(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(user_id) => user_id,
        Err(_) => {
            resp.render(ResponseResult {
                code: 401,
                message: "unauthorized.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
    };
    let form = match req.parse_json::<SetAdminReq>().await {
        Ok(form) => form,
        Err(_) => {
            resp.render(ResponseResult {
                code: 400,
                message: "set admin parameter mismatch.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
    };
    let user_relationship1 =
        match UserRelationship::get_user_id_peer_id(user_id as i64, form.group_id as i64).await {
            Ok(user_relationship) => user_relationship,
            Err(_) => {
                resp.render(ResponseResult {
                    code: 400,
                    message: "user1 not in the group.",
                    timestamp: Local::now(),
                    data: (),
                });
                return;
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
        resp.render(ResponseResult {
            code: 400,
            message: "you are not admin.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut user_relationship2 = match UserRelationship::get_user_id_peer_id(
        form.user_id as i64,
        form.group_id as i64,
    )
    .await
    {
        Ok(user_relationship) => user_relationship,
        Err(_) => {
            resp.render(ResponseResult {
                code: 400,
                message: "user2 not in the group.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
    };
    let mut group = match Group::get_group_id(form.group_id as i64).await {
        Ok(group) => group,
        Err(_) => {
            resp.render(ResponseResult {
                code: 400,
                message: "group not found.",
                timestamp: Local::now(),
                data: (),
            });
            return;
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
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    if let Err(e) = group.update().await {
        error!("update group error: {}", e);
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    });
}
