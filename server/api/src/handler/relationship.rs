use std::time::Duration;

use chrono::Local;
use lib::{
    entity::{Msg, Type},
    util::who_we_are,
};
use salvo::handler;

use crate::{
    cache::{get_redis_ops, ADD_FRIEND, USER_RELATIONSHIP},
    model::relationship::{UserRelationship, UserRelationshipStatus},
    rpc::get_rpc_client,
    sql::DELETE_AT,
};

use super::{verify_user, ResponseResult};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AddFriendReq {
    peer_id: u64,
    remark: String,
}

#[handler]
pub(crate) async fn add_friend(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let form: std::result::Result<AddFriendReq, _> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: form.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let form = form.unwrap();
    let key = format!("{}{}-{}", ADD_FRIEND, user_id, form.peer_id);
    let res = redis_ops.get::<String>(&key).await;
    if res.is_ok() {
        resp.render(ResponseResult {
            code: 400,
            message: "already send add friend request.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    _ = redis_ops
        .set_exp(&key, &form.remark, Duration::from_secs(60 * 60 * 24 * 7))
        .await;
    let mut msg = Msg::text(user_id, form.peer_id, 0, &form.remark);
    msg.set_type(Type::AddFriend);
    let mut rpc_client = get_rpc_client().await;
    let res = rpc_client.call_push_msg(&msg).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: res.err().unwrap().to_string().as_str(),
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ConfirmAddFriendReq {
    peer_id: u64,
    passed: bool,
}

#[handler]
pub(crate) async fn confirm_add_friend(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let form: std::result::Result<ConfirmAddFriendReq, _> = req.parse_json().await;
    if form.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: form.err().unwrap().to_string().as_str(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let form = form.unwrap();
    let key = format!("{}{}-{}", ADD_FRIEND, form.peer_id, user_id);
    let res = redis_ops.get::<String>(&key).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "no add friend request.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    _ = redis_ops.del(&key).await;
    let user_relationship1 = UserRelationship {
        id: 0,
        user_id: user_id as i64,
        peer_id: form.peer_id as i64,
        remark: "".to_string(),
        status: UserRelationshipStatus::Normal,
        classification: "".to_string(),
        tag_list: vec![],
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
        create_at: Local::now(),
        update_at: Local::now(),
        delete_at: DELETE_AT.clone(),
    };
    let res1 = user_relationship1.insert().await;
    if res1.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res2 = user_relationship2.insert().await;
    if res2.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let remark = res.unwrap();
    let mut msg = Msg::text(user_id, form.peer_id, 0, &remark);
    msg.set_type(Type::AddFriend);
    let mut rpc_client = get_rpc_client().await;
    let res = rpc_client.call_push_msg(&msg).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: res.err().unwrap().to_string().as_str(),
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

/// only both sides all invoke this method, the relationship will be dropped.
#[handler]
pub(crate) async fn delete_friend(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let peer_id: Option<u64> = req.query("peer_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer_id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let res1 = UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await;
    if res1.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "relationship already unlocked.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res2 = UserRelationship::get_user_id_peer_id(peer_id as i64, user_id as i64).await;
    if res2.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "relationship already unlocked.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    _ = res1.unwrap().delete().await;
    _ = res2.unwrap().delete().await;
    let mut msg = Msg::text(user_id, peer_id, 0, "we have broken up.");
    msg.set_type(Type::RemoveFriend);
    let mut rpc_client = get_rpc_client().await;
    let res = rpc_client.call_push_msg(&msg).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: res.err().unwrap().to_string().as_str(),
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct FriendListResp {
    peer_id: u64,
    remark: String,
    status: u8,
    classification: String,
    tag_list: Vec<String>,
}

#[handler]
pub(crate) async fn get_friend_list(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let number: Option<u64> = req.query("number");
    if number.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "number is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let number = number.unwrap();
    let offset: Option<u64> = req.query("offset");
    if offset.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "offset is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let offset = offset.unwrap();
    if number > 100 {
        resp.render(ResponseResult {
            code: 400,
            message: "number must less than 100.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res = UserRelationship::get_user_id(user_id as i64, number as i64, offset as i64).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "no relationship.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res = res.unwrap();
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
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: list,
    });
}

/// 1. not friend
/// 2. friend but with different status, such as: normal, best friend, block, lover...
#[handler]
pub(crate) async fn get_peer_relationship(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let peer_id: Option<u64> = req.query("peer_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer_id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let res = UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "not friend.",
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
        data: res.status as u8,
    });
}

/// only work on user and peer is already friend.
#[handler]
pub(crate) async fn update_relationship(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let peer_id: Option<u64> = req.query("peer_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer_id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let status: Option<u8> = req.query("status");
    if status.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "status is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let status = status.unwrap();
    let key = format!("{}{}", USER_RELATIONSHIP, who_we_are(user_id, peer_id));
    let res = redis_ops.get::<u8>(&key).await;
    if res.is_err() {
        _ = redis_ops
            .set_exp(&key, &status, Duration::from_secs(60 * 60 * 24 * 7))
            .await;
        let mut msg = Msg::text(user_id, peer_id, 0, &status.to_string());
        msg.set_type(Type::SetRelationship);
        let mut rpc_client = get_rpc_client().await;
        let res = rpc_client.call_push_msg(&msg).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 400,
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
        return;
    }
    let res = res.unwrap();
    if res != status {
        _ = redis_ops.del(&key).await;
        resp.render(ResponseResult {
            code: 200,
            message: "ok.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res1 = UserRelationship::get_user_id_peer_id(user_id as i64, peer_id as i64).await;
    if res1.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "not friend.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut res1 = res1.unwrap();
    let res2 = UserRelationship::get_user_id_peer_id(peer_id as i64, user_id as i64).await;
    if res2.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "not friend.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut res2 = res2.unwrap();
    res1.status = UserRelationshipStatus::from(status);
    res2.status = UserRelationshipStatus::from(status);
    let res1 = res1.update().await;
    if res1.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res2 = res2.update().await;
    if res2.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    _ = redis_ops.del(&key).await;
    let mut msg = Msg::text(user_id, peer_id, 0, &status.to_string());
    msg.set_type(Type::SetRelationship);
    let mut rpc_client = get_rpc_client().await;
    let res = rpc_client.call_push_msg(&msg).await;
    if res.is_err() {
        resp.render(ResponseResult {
            code: 400,
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
