use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};
use redis::RedisResult;
use crate::persistence::redis_ops::RedisOps;

#[derive(Debug, Serialize, Deserialize)]
struct SyncStruct {
    // seq_num
    s: u64,
    // is_backing
    b: bool,
    // length
    l: usize,
}

pub async fn work(msg: &Msg, connection_map: &mut MsgMap, redis_ops: &mut RedisOps) -> Option<Vec<Msg>> {
    match msg.head.typ {
        msg::Type::Sync => {
            let params: serde_json::Result<SyncStruct> = serde_json::from_slice(msg.payload.as_slice());
            if let Err(e) = params {
                error!("parse params failed: {}", e);
                let mut result = Vec::with_capacity(1);
                result.push(Msg::err_msg_str(0, msg.head.sender, "parse params failed"));
                return Some(result);
            }
            let mut params = params.unwrap();
            // 构建key
            let mut key = util::base::who_we_are(msg.head.sender, msg.head.receiver);
            key.push_str("-msg_channel");
            // 默认情况
            if params.s == 0 {
                params.s = u64::MAX;
                params.b = true;
                params.l = 20;
            }
            let list: RedisResult<Vec<Msg>> = redis_ops.peek_sort_queue_more_and_more(key, 0, params.l, params.s as f64, params.b).await;
            if let Err(e) = list {
                error!("redis read error: {}", e);
                let mut result = Vec::with_capacity(1);
                result.push(Msg::err_msg_str(0, msg.head.sender, "redis read error"));
                return Some(result);
            }
            let mut list = list.unwrap();
            // 结果列表
            let mut result: Vec<Msg> = Vec::with_capacity(params.l + 1);
            result.push(Msg::text(0, msg.head.sender, list.len().to_string()));
            result.append(& mut list);
            Some(result)
        }
        msg::Type::Auth => {
            let mut result = Vec::with_capacity(1);
            result.push(Msg::text_str(0, msg.head.sender, "ok"));
            Some(result)
        }
        _ => {
            None
        }
    }
}