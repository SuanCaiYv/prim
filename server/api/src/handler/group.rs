use std::time::Duration;

use chrono::Local;
use lib::entity::{Msg, Type, GROUP_ID_THRESHOLD};
use salvo::{handler, http::ParseError, Request, Response};
use serde_json::json;

use crate::cache::CHECK_CODE;
use crate::model::group::GroupStatus;
use crate::sql::DELETE_AT;
use crate::{
    cache::{get_redis_ops, JOIN_GROUP},
    model::group::{Group, UserGroupList, UserGroupRole},
    rpc::get_rpc_client,
};

use super::{verify_user, ResponseResult};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JoinGroupReq {
    group_id: f64,
    check_code: String,
}

/// invoked by someone who wants to join a group
#[handler]
pub(crate) async fn join_group(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form: Result<JoinGroupReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "join group parameters mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let join_group_key = format!("{}{}_{}", JOIN_GROUP, user_id, form.group_id as u64);
    match redis_ops.get::<String>(&join_group_key).await {
        Ok(_) => {
            resp.render(ResponseResult {
                code: 400,
                message: "you have already applied for join this group.",
                timestamp: Local::now(),
                data: "",
            });
            return;
        }
        Err(_) => {
            let result = redis_ops
                .set_exp(
                    &join_group_key,
                    &form.check_code,
                    Duration::from_secs(3 * 24 * 60 * 60),
                )
                .await;
            if result.is_err() {
                resp.render(ResponseResult {
                    code: 500,
                    message: "internal server error.",
                    timestamp: Local::now(),
                    data: "",
                });
                return;
            }
            let mut rpc_client = get_rpc_client().await;
            let group = Group::get_group_id(form.group_id as i64).await;
            if group.is_err() {
                resp.render(ResponseResult {
                    code: 500,
                    message: "internal server error.",
                    timestamp: Local::now(),
                    data: "",
                });
                return;
            }
            let group = group.unwrap();
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
                let res = rpc_client.call_push_msg(&msg).await;
                if res.is_err() {
                    resp.render(ResponseResult {
                        code: 500,
                        message: "internal server error.",
                        timestamp: Local::now(),
                        data: "",
                    });
                    return;
                }
            }
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: "",
            });
            return;
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
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let group_id: Option<u64> = req.param("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "leave group parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let group_id = group_id.unwrap();
    let user_group_list =
        UserGroupList::get_user_id_group_id(user_id as i64, group_id as i64).await;
    if user_group_list.is_err() {
        resp.render(ResponseResult {
            code: 500,
            // todo: change to "user not in group"
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list = user_group_list.unwrap();
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
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
            .as_u64()
            .unwrap();
        let mut msg = Msg::raw(user_id, admin_user_id, 0, &user_id.to_string().as_bytes());
        msg.set_type(Type::LeaveGroup);
        let res = rpc_client.call_push_msg(&msg).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: "",
            });
            return;
        }
    }
    _ = user_group_list.delete().await;
    if user_group_list.role == UserGroupRole::Admin {
        let mut admin_list = group.admin_list;
        admin_list.retain(|admin| {
            admin
                .as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_u64()
                .unwrap()
                != user_id
        });
        group.admin_list = admin_list;
    }
    if user_group_list.role == UserGroupRole::Member {
        let mut member_list = group.member_list;
        member_list.retain(|member| {
            member
                .as_object()
                .unwrap()
                .get("user_id")
                .unwrap()
                .as_u64()
                .unwrap()
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
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: "",
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateGroupReq {
    group_name: String,
    check_code: String,
}

#[handler]
pub(crate) async fn create_group(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form: Result<CreateGroupReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "create group parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let mut group_id;
    loop {
        group_id = fastrand::u64(GROUP_ID_THRESHOLD..1 << 46);
        let group = Group::get_group_id(group_id as i64).await;
        if group.is_err() {
            break;
        }
    }
    _ = redis_ops
        .set(&format!("{}{}", CHECK_CODE, group_id), &form.check_code)
        .await;
    let group = Group {
        id: 0,
        group_id: group_id as i64,
        name: form.group_name,
        avatar: "".to_string(),
        admin_list: vec![json!({
            "user_id": user_id,
            "nickname": user_id.to_string(),
        })],
        member_list: vec![],
        status: GroupStatus::Normal,
        info: json!({}),
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    let res = group.insert().await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list = UserGroupList {
        id: 0,
        user_id: user_id as i64,
        group_id: group_id as i64,
        role: UserGroupRole::Admin,
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    let res = user_group_list.insert().await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: group_id.to_string(),
    });
    todo!()
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
    let group_id: Option<u64> = req.param("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "group id is required.",
            timestamp: Local::now(),
            data: "",
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
            data: "",
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
    group_id: f64,
    name: Option<String>,
    avatar: Option<String>,
    info: Option<serde_json::Value>,
}

#[handler]
pub(crate) async fn update_group_info(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form: Result<UpdateGroupInfoReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "update group info parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let group = Group::get_group_id(form.group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 404,
            message: "group not found.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let mut group = group.unwrap();
    let user_group_list =
        UserGroupList::get_user_id_group_id(user_id as i64, form.group_id as i64).await;
    if user_group_list.is_err() {
        resp.render(ResponseResult {
            code: 403,
            message: "you are not in this group.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list = user_group_list.unwrap();
    if user_group_list.role != UserGroupRole::Admin {
        resp.render(ResponseResult {
            code: 403,
            message: "you are not admin of this group.",
            timestamp: Local::now(),
            data: "",
        });
        return;
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
    let res = group.update().await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: "",
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupUserListReq {
    group_id: f64,
    user_role: u8,
    offset: f64,
    limit: f64,
}

#[handler]
pub(crate) async fn get_group_user_list(req: &mut Request, resp: &mut Response) {
    let form: Result<GroupUserListReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "get group user list parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let group = Group::get_group_id(form.group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 404,
            message: "group not found.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let group = group.unwrap();
    let role = UserGroupRole::from(form.user_role);
    match role {
        UserGroupRole::Admin => {
            let offset = if form.offset as usize > group.admin_list.len() {
                group.admin_list.len()
            } else {
                form.offset as usize
            };
            let limit = if form.limit as usize + offset > group.admin_list.len() {
                group.admin_list.len()
            } else {
                form.limit as usize + offset
            };
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: group.admin_list.as_slice()[offset..limit].to_vec(),
            });
        }
        UserGroupRole::Member => {
            let offset = if form.offset as usize > group.member_list.len() {
                group.member_list.len()
            } else {
                form.offset as usize
            };
            let limit = if form.limit as usize + offset > group.member_list.len() {
                group.member_list.len()
            } else {
                form.limit as usize + offset
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
                data: "",
            });
            return;
        }
    }
    todo!()
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupUserReq {
    group_id: f64,
    user_id: f64,
    reason: String,
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
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let peer_id: Option<u64> = req.param("user_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let group_id: Option<u64> = req.param("group_id");
    if group_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let group_id = group_id.unwrap();
    let reason: Option<&str> = req.param("reason");
    if reason.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "remove member parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let reason = reason.unwrap();
    let user_group_list =
        UserGroupList::get_user_id_group_id(user_id as i64, group_id as i64).await;
    if user_group_list.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "operation user id mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list = user_group_list.unwrap();
    if user_group_list.role != UserGroupRole::Admin {
        resp.render(ResponseResult {
            code: 400,
            message: "operation user is not admin.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let group = Group::get_group_id(group_id as i64).await;
    if group.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "group id mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let mut group = group.unwrap();
    let peer_group_list =
        UserGroupList::get_user_id_group_id(peer_id as i64, group_id as i64).await;
    if peer_group_list.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer user id mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let peer_group_list = peer_group_list.unwrap();
    // not allow to remove admin.
    if peer_group_list.role == UserGroupRole::Admin {
        resp.render(ResponseResult {
            code: 400,
            message: "peer user is admin.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let member_list = &mut group.member_list;
    member_list.retain(|x| {
        x.as_object()
            .unwrap()
            .get("user_id")
            .unwrap()
            .as_u64()
            .unwrap()
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
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: "",
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ApproveJoinReq {
    group_id: f64,
    user_id: f64,
    approve: bool,
}

/// invoked for admin user to approve someone to join the group.
#[handler]
pub(crate) async fn approve_join(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form: Result<ApproveJoinReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "approve join parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    if form.approve {
        let user_group_list = UserGroupList {
            id: 0,
            user_id: form.user_id as i64,
            group_id: form.group_id as i64,
            role: UserGroupRole::Member,
            create_at: Local::now(),
            update_at: Local::now(),
            delete_at: DELETE_AT.clone(),
        };
        let res = user_group_list.insert().await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 400,
                message: "user may already in the group.",
                timestamp: Local::now(),
                data: "",
            });
            return;
        } else {
            let mut group = Group::get_group_id(form.group_id as i64).await.unwrap();
            let mut member_list = group.member_list;
            member_list.push(json!({
                "user_id": form.user_id,
                "remark": form.user_id.to_string(),
            }));
            group.member_list = member_list;
            _ = group.update().await;
        }
        let mut msg = Msg::raw(
            user_id,
            form.user_id as u64,
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
        let res = rpc_client.call_push_msg(&msg).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: "",
            });
            return;
        }
    } else {
        let mut msg = Msg::raw(
            user_id,
            form.user_id as u64,
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
        let res = rpc_client.call_push_msg(&msg).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: "",
            });
            return;
        }
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: "",
    });
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SetAdminReq {
    group_id: f64,
    user_id: f64,
    is_admin: bool,
}

/// invoked for admin user to set/unset the admin of the group.
#[handler]
pub(crate) async fn set_admin(req: &mut Request, resp: &mut Response) {
    let mut redis_ops = get_redis_ops().await;
    let user_id = verify_user(req, &mut redis_ops).await;
    if user_id.is_err() {
        resp.render(ResponseResult {
            code: 401,
            message: user_id.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_id = user_id.unwrap();
    let form: Result<SetAdminReq, ParseError> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "set admin parameter mismatch.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let form = form.unwrap();
    let user_group_list =
        UserGroupList::get_user_id_group_id(user_id as i64, form.group_id as i64).await;
    if user_group_list.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "you are not in the group.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list = user_group_list.unwrap();
    if user_group_list.role != UserGroupRole::Admin {
        resp.render(ResponseResult {
            code: 400,
            message: "you are not admin of the group.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let user_group_list =
        UserGroupList::get_user_id_group_id(form.user_id as i64, form.group_id as i64).await;
    if user_group_list.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "user is not in the group.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    let mut user_group_list = user_group_list.unwrap();
    if form.is_admin {
        user_group_list.role = UserGroupRole::Admin;
    } else {
        user_group_list.role = UserGroupRole::Member;
    }
    let res = user_group_list.update().await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: "",
        });
        return;
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: "",
    });
    todo!()
}
