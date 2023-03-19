use chrono::Local;
use lib::{
    entity::{Msg, Type},
    util::{who_we_are, timestamp},
    Result,
};
use salvo::{handler, http::ParseError};

use crate::{
    cache::{get_redis_ops, LAST_ONLINE_TIME, LAST_READ, MSG_CACHE, USER_INBOX},
    model::msg::Message,
    rpc::get_rpc_client,
};

use super::{verify_user, ResponseResult};

/// depends on certain client.
/// this method will return all users who have sent message to this user when the user is offline.
/// by this we can promise that no user-peer list will be lost.
/// so the blow method can get passed messages.
#[handler]
pub(crate) async fn inbox(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    // todo device dependency
    let mut last_online_time = match redis_ops
        .get::<u64>(&format!("{}{}", LAST_ONLINE_TIME, user_id))
        .await
    {
        Ok(v) => v,
        Err(_) => timestamp() - 5 * 365 * 24 * 60 * 60 * 1000,
    };
    let user_list: Result<Vec<u64>> = redis_ops
        .peek_sort_queue_more(
            &format!("{}{}", USER_INBOX, user_id),
            0,
            u32::MAX as usize,
            last_online_time as f64,
            f64::MAX,
            false,
        )
        .await;
    if user_list.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_list = user_list.unwrap();
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: user_list,
    });
}

/// a state cross multi-client.
#[handler]
pub(crate) async fn unread(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let peer_id = req.query::<u64>("peer_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let last_read_seq_num: Result<u64> = redis_ops
        .get(&format!("{}{}-{}", LAST_READ, user_id, peer_id))
        .await;
    if last_read_seq_num.is_err() {
        resp.render(ResponseResult {
            code: 200,
            message: "ok.",
            timestamp: Local::now(),
            data: 0,
        });
        return;
    }
    let last_read_seq_seq = last_read_seq_num.unwrap();
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: last_read_seq_seq,
    });
}

#[handler]
pub(crate) async fn update_unread(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let peer_id = req.query::<u64>("peer_id");
    if peer_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer id is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    // todo update other client's last_read.
    let last_read_seq = req.query::<u64>("last_read_seq");
    if last_read_seq.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "last read seq is required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let last_read_seq = last_read_seq.unwrap();
    let peer_id = peer_id.unwrap();
    if let Err(_) = redis_ops
        .set(
            &format!("{}{}-{}", LAST_READ, user_id, peer_id),
            &last_read_seq,
        )
        .await
    {
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

#[handler]
pub(crate) async fn history_msg(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let old_seq_num: Option<u64> = req.query("old_seq_num");
    let new_seq_num: Option<u64> = req.query("new_seq_num");
    if peer_id.is_none() || old_seq_num.is_none() || new_seq_num.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer id, old seq num, new seq num and are required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    // range is [old, new)
    let old_seq_num = old_seq_num.unwrap();
    let mut new_seq_num = new_seq_num.unwrap();
    if new_seq_num == 0 {
        new_seq_num = old_seq_num + 99 + 1;
    }
    let expected_size = (new_seq_num - old_seq_num) as usize;
    if expected_size > 100 {
        resp.render(ResponseResult {
            code: 400,
            message: "too many messages.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let id_key = who_we_are(user_id, peer_id);
    let msg_list: Result<Vec<Msg>> = redis_ops
        .peek_sort_queue_more(
            &format!("{}{}", MSG_CACHE, id_key),
            0,
            100,
            old_seq_num as f64,
            new_seq_num as f64,
            true,
        )
        .await;
    if msg_list.is_err() {
        resp.render(ResponseResult {
            code: 500,
            message: "internal server error.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let mut msg_list = msg_list.unwrap();
    if msg_list.len() < expected_size {
        let new_seq_num = new_seq_num - (msg_list.len() as u64);
        let list = Message::get_by_user_and_peer(
            user_id as i64,
            peer_id as i64,
            old_seq_num as i64,
            new_seq_num as i64,
        )
            .await;
        if list.is_err() {
            resp.render(ResponseResult {
                code: 500,
                message: "internal server error.",
                timestamp: Local::now(),
                data: (),
            });
            return;
        }
        let list = list.unwrap();
        let list = list.iter().map(|x| x.into()).collect::<Vec<Msg>>();
        msg_list.extend(list);
    }
    resp.render(ResponseResult {
        code: 200,
        message: "ok.",
        timestamp: Local::now(),
        data: msg_list,
    });
}

#[handler]
pub(crate) async fn withdraw(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let seq_num: Option<u64> = req.query("old_seq_num");
    if peer_id.is_none() || seq_num.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer id, old seq num, new seq num and number are required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let peer_id = peer_id.unwrap();
    let seq_num = seq_num.unwrap();
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
            new_msg.set_seq_num(msg.seq_num());
            _ = redis_ops
                .remove_sort_queue_data(&user_peer_key, msg.seq_num() as f64)
                .await;
            _ = redis_ops
                .push_sort_queue(&user_peer_key, &new_msg, new_msg.seq_num() as f64)
                .await;
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            });
            return;
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
    msg.set_seq_num(seq_num);
    msg.set_type(Type::Withdraw);
    if let Err(_) = rpc_client.call_push_msg(&msg).await {
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct EditReq {
    peer_id: u64,
    seq_num: u64,
    new_text: String,
}

/// only allow message (type == text) to be edited.
#[handler]
pub(crate) async fn edit(req: &mut salvo::Request, resp: &mut salvo::Response) {
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
    let edit_req: std::result::Result<EditReq, ParseError> = req.parse_json().await;
    if edit_req.is_err() {
        resp.render(ResponseResult {
            code: 400,
            message: "peer id, seq num, and new_text are required.",
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let edit_req = edit_req.unwrap();
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
            new_msg.set_seq_num(msg.seq_num());
            _ = redis_ops
                .remove_sort_queue_data(&user_peer_key, msg.seq_num() as f64)
                .await;
            _ = redis_ops
                .push_sort_queue(&user_peer_key, &new_msg, new_msg.seq_num() as f64)
                .await;
            resp.render(ResponseResult {
                code: 200,
                message: "ok.",
                timestamp: Local::now(),
                data: (),
            });
            return;
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
            message.payload = base64::encode(&edit_req.new_text);
            message.extension = "".to_string();
            _ = message.update().await;
        }
    }
    let mut rpc_client = get_rpc_client().await;
    let mut msg = Msg::text(user_id, edit_req.seq_num, 0, &edit_req.new_text);
    msg.set_seq_num(edit_req.seq_num);
    msg.set_type(Type::Edit);
    if let Err(_) = rpc_client.call_push_msg(&msg).await {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::json;
    use lib::entity::Msg;

    #[test]
    fn test() {
        let val = u64::MAX;
        let mut map = HashMap::new();
        map.insert("a", val);
        println!("{}", serde_json::to_string(&map).unwrap());
    }
}
