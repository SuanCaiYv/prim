use base64::Engine;
use chrono::Local;
use lib::{
    entity::{Msg, Type, GROUP_ID_THRESHOLD},
    util::{timestamp, who_we_are},
    Result,
};
use salvo::handler;
use tracing::error;

use crate::{
    cache::{get_redis_ops, LAST_ONLINE_TIME, LAST_READ, MSG_CACHE, USER_INBOX},
    error::HandlerError,
    model::msg::Message,
    rpc::get_rpc_client,
};

use super::{verify_user, HandlerResult, ResponseResult};

/// depends on certain client.
/// this method will return all users who have sent message to this user when the user is offline.
/// by this we can promise that no user-peer list will be lost.
/// so the blow method can get passed messages.
#[handler]
pub(crate) async fn inbox(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, Vec<u64>> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    // todo device dependency
    let last_online_time = match redis_ops
        .get::<u64>(&format!("{}{}", LAST_ONLINE_TIME, user_id))
        .await
    {
        Ok(v) => 1,
        Err(_) => timestamp() - 5 * 365 * 24 * 60 * 60 * 1000,
    };
    let user_list = match redis_ops
        .peek_sort_queue_more::<u64>(
            &format!("{}{}", USER_INBOX, user_id),
            0,
            u32::MAX as usize,
            last_online_time as f64,
            f64::MAX,
            false,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("get user inbox failed: {}", e);
            return Err(HandlerError::InternalError("internal error".to_string()));
        }
    };
    println!("{:?}", user_list);
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: user_list,
    })
}

/// a state cross multi-client.
#[handler]
pub(crate) async fn unread(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, u64> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let last_read_seq_num = match redis_ops
        .get::<u64>(&format!("{}{}-{}", LAST_READ, user_id, peer_id))
        .await
    {
        Ok(v) => v,
        Err(_) => 0,
    };
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: last_read_seq_num,
    })
}

#[handler]
pub(crate) async fn update_unread(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    // todo update other client's last_read.
    let last_read_seq = match req.query::<u64>("last_read_seq") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "last read seq is required.".to_string(),
            ))
        }
    };
    if let Err(_) = redis_ops
        .set(
            &format!("{}{}-{}", LAST_READ, user_id, peer_id),
            &last_read_seq,
        )
        .await
    {
        error!("update unread failed.");
        return Err(HandlerError::InternalError("internal error".to_string()));
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

/// to_seq_num == 0: client don't know the newest seq_num, but it will provide it's local latest seq_num.
///
/// to_seq_num != 0: client have synchronized the msg list and wants more msgs.
///
/// the logic of msg_history is: find message from cache firstly, if it's empty of less than number expected, an db query will be launched.
///
/// for `seq_num == 0`, it's a little complexly for the logic:
///
/// - get all new msg from cache, if the oldest seq_num match the parameter, returned.
/// - try to get remained msgs from db.
#[handler]
pub(crate) async fn history_msg(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, Vec<Msg>> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let from_seq_num = match req.query::<u64>("from_seq_num") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "from_seq_num is required.".to_string(),
            ))
        }
    };
    let to_seq_num = match req.query::<u64>("to_seq_num") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "to_seq_num is required.".to_string(),
            ))
        }
    };
    let expected_size = if to_seq_num == 0 {
        100
    } else {
        (to_seq_num - from_seq_num) as usize
    };
    if expected_size > 100 {
        return Err(HandlerError::RequestMismatch(
            400,
            "expected size is too large.".to_string(),
        ));
    }
    let id_key = if peer_id >= GROUP_ID_THRESHOLD {
        who_we_are(peer_id, peer_id)
    } else {
        who_we_are(user_id, peer_id)
    };
    let cache_from_seq_num = from_seq_num as f64;
    let mut cache_to_seq_num = to_seq_num as f64;
    let mut db_from_seq_num = from_seq_num as i64;
    let mut db_to_seq_num = to_seq_num as i64;
    if to_seq_num == 0 {
        cache_to_seq_num = f64::MAX;
        db_to_seq_num = i64::MAX;
    }
    println!("{} {}", cache_from_seq_num, cache_to_seq_num);
    let cache_list = redis_ops
        .peek_sort_queue_more::<Msg>(
            &format!("{}{}", MSG_CACHE, id_key),
            0,
            expected_size,
            cache_from_seq_num,
            cache_to_seq_num,
            false,
        )
        .await;
    if cache_list.is_err() {
        error!("redis error: {}", cache_list.err().unwrap());
        return Err(HandlerError::InternalError("internal error".to_string()));
    }
    println!("{:?}", cache_list);
    let cache_list = cache_list.unwrap();
    if cache_list.len() == expected_size {
        return Ok(ResponseResult {
            code: 200,
            message: "ok.",
            timestamp: Local::now(),
            data: cache_list,
        });
    }
    if cache_list.len() > 0 {
        db_to_seq_num = cache_list[0].seqnum() as i64;
        if to_seq_num == 0 {
            db_from_seq_num = db_to_seq_num - ((expected_size - cache_list.len()) as i64);
        }
    } else {
        if to_seq_num == 0 {
            db_from_seq_num = db_to_seq_num - expected_size as i64;
        }
    }
    let db_list = Message::get_by_user_and_peer(
        user_id as i64,
        peer_id as i64,
        db_from_seq_num,
        db_to_seq_num,
    )
    .await;
    if db_list.is_err() {
        error!("db error: {}", db_list.err().unwrap());
        return Err(HandlerError::InternalError("internal error".to_string()));
    }
    let db_list = db_list.unwrap();
    let mut list = db_list.iter().map(|x| x.into()).collect::<Vec<Msg>>();
    list.extend(cache_list);
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: list,
    })
}

#[handler]
pub(crate) async fn withdraw(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    let peer_id = match req.query::<u64>("peer_id") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "peer id is required.".to_string(),
            ))
        }
    };
    let seq_num = match req.query::<u64>("old_seq_num") {
        Some(v) => v,
        None => {
            return Err(HandlerError::ParameterMismatch(
                "old_seq_num is required.".to_string(),
            ))
        }
    };
    let id_key = who_we_are(user_id, peer_id);
    let user_peer_key = format!("{}{}", MSG_CACHE, id_key);
    let res: Result<Vec<Msg>> = redis_ops
        .peek_sort_queue_more(&user_peer_key, 0, 1, seq_num as f64, seq_num as f64, true)
        .await;
    if let Ok(res) = res {
        if res.len() > 0 {
            let msg = &res[0];
            let mut new_msg = Msg::raw(msg.sender(), msg.receiver(), msg.node_id(), &[]);
            new_msg.set_type(Type::Withdraw);
            new_msg.set_seqnum(msg.seqnum());
            _ = redis_ops
                .remove_sort_queue_data(&user_peer_key, msg.seqnum() as f64)
                .await;
            _ = redis_ops
                .push_sort_queue(&user_peer_key, &new_msg, new_msg.seqnum() as f64)
                .await;
            return Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            });
        }
    }
    let message_list = Message::get_by_user_and_peer(
        user_id as i64,
        peer_id as i64,
        seq_num as i64,
        seq_num as i64,
    )
    .await;
    if let Ok(mut message_list) = message_list {
        if message_list.len() > 0 {
            let message = &mut message_list[0];
            message.typ = Type::Withdraw;
            message.payload = "".to_string();
            message.extension = "".to_string();
            _ = message.update().await;
        } else {
            // todo
        }
    }
    let mut rpc_client = get_rpc_client().await;
    let mut msg = Msg::raw(user_id, peer_id, 0, &[]);
    msg.set_seqnum(seq_num);
    msg.set_type(Type::Withdraw);
    if let Err(e) = rpc_client.call_push_msg(&msg).await {
        error!("rpc call push msg error: {}", e);
        return Err(HandlerError::InternalError("internal error".to_string()));
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct EditReq {
    peer_id: u64,
    seq_num: u64,
    new_text: String,
}

/// only allow message (type == text) to be edited.
#[handler]
pub(crate) async fn edit(
    req: &mut salvo::Request,
    _resp: &mut salvo::Response,
) -> HandlerResult<'static, ()> {
    let mut redis_ops = get_redis_ops().await;
    let user_id = match verify_user(req, &mut redis_ops).await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::RequestMismatch(
                401,
                "unauthorized".to_string(),
            ))
        }
    };
    let edit_req = match req.parse_json::<EditReq>().await {
        Ok(v) => v,
        Err(_e) => {
            return Err(HandlerError::ParameterMismatch(
                "peer id, seq num, and new_text are required.".to_string(),
            ))
        }
    };
    let id_key = who_we_are(user_id, edit_req.peer_id);
    let user_peer_key = format!("{}{}", MSG_CACHE, id_key);
    let res: Result<Vec<Msg>> = redis_ops
        .peek_sort_queue_more(
            &user_peer_key,
            0,
            1,
            edit_req.seq_num as f64,
            edit_req.seq_num as f64,
            true,
        )
        .await;
    if let Ok(res) = res {
        if res.len() > 0 {
            let msg = &res[0];
            let mut new_msg = Msg::text(
                msg.sender(),
                msg.receiver(),
                msg.node_id(),
                &edit_req.new_text,
            );
            new_msg.set_type(Type::Edit);
            new_msg.set_seqnum(msg.seqnum());
            _ = redis_ops
                .remove_sort_queue_data(&user_peer_key, msg.seqnum() as f64)
                .await;
            _ = redis_ops
                .push_sort_queue(&user_peer_key, &new_msg, new_msg.seqnum() as f64)
                .await;
            return Ok(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            });
        }
    }
    let message_list = Message::get_by_user_and_peer(
        user_id as i64,
        edit_req.peer_id as i64,
        edit_req.seq_num as i64,
        edit_req.seq_num as i64,
    )
    .await;
    if let Ok(mut message_list) = message_list {
        if message_list.len() > 0 {
            let message = &mut message_list[0];
            message.typ = Type::Edit;
            let engine = base64::engine::GeneralPurpose::new(
                &base64::alphabet::URL_SAFE,
                base64::engine::general_purpose::PAD,
            );
            message.payload = engine.encode(&edit_req.new_text);
            message.extension = "".to_string();
            _ = message.update().await;
        }
    }
    let mut rpc_client = get_rpc_client().await;
    let mut msg = Msg::text(user_id, edit_req.seq_num, 0, &edit_req.new_text);
    msg.set_seqnum(edit_req.seq_num);
    msg.set_type(Type::Edit);
    if let Err(e) = rpc_client.call_push_msg(&msg).await {
        error!("rpc call push msg error: {}", e);
        return Err(HandlerError::InternalError("internal error".to_string()));
    }
    Ok(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: (),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn test() {
        let val = u64::MAX;
        let mut map = HashMap::new();
        map.insert("a", val);
        println!("{}", serde_json::to_string(&map).unwrap());
    }
}
