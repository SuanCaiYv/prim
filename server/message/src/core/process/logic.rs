use tracing::{debug, error};
use serde::{Deserialize, Serialize};
use crate::entity::msg;
use crate::util::base;
use crate::core::net;
use crate::util;

#[derive(Debug, Serialize, Deserialize)]
struct SyncStruct {
    // seq_num
    s: u64,
    // is_backing
    b: bool,
    // length
    l: usize,
}

pub async fn process(msg: &mut msg::Msg, redis_ops: &mut net::RedisOps) -> std::io::Result<Vec<msg::Msg>> {
    msg.head.timestamp = util::base::timestamp();
    match msg.head.typ {
        msg::Type::Sync => {
            let params: serde_json::Result<SyncStruct> = serde_json::from_slice(msg.payload.as_slice());
            if let Err(e) = params {
                error!("parse params failed: {}", e);
                let mut result = Vec::with_capacity(1);
                result.push(msg::Msg::err_msg_str(0, msg.head.sender, "parse params failed"));
                return Ok(result);
            }
            let mut params = params.unwrap();
            // 构建key
            let mut key = base::who_we_are(msg.head.sender, msg.head.receiver);
            key.push_str("-msg_channel");
            // 默认情况
            if params.s == 0 {
                params.s = u64::MAX;
                params.b = true;
                params.l = 20;
            }
            let list: redis::RedisResult<Vec<msg::Msg>> = redis_ops.peek_sort_queue_more(key, 0, params.l, params.b, params.s as f64).await;
            if let Err(e) = list {
                error!("redis read sync error: {}", e);
                let mut result = Vec::with_capacity(1);
                result.push(msg::Msg::err_msg_str(0, msg.head.sender, "redis read error"));
                return Ok(result);
            }
            let mut list = list.unwrap();
            list.reverse();
            // 结果列表
            let mut result: Vec<msg::Msg> = Vec::with_capacity(list.len()+1);
            result.push(msg::Msg::sync_head(msg.head.sender, msg.head.receiver, list.len()));
            for mut msg in list {
                msg.wrap_sync();
                result.push(msg);
            }
            Ok(result)
        },
        msg::Type::Box => {
            let msg_box_channel = format!("{}-msg_box", msg.head.sender);
            // 注意这里有收件箱大小限制
            let list: redis::RedisResult<Vec<(u64, f64)>> = redis_ops.peek_sort_queue_more_with_score(msg_box_channel, 0, 1 << 16, true, f64::MAX).await;
            if let Err(e) = list {
                error!("redis read box error: {}", e);
                let mut result = Vec::with_capacity(1);
                result.push(msg::Msg::err_msg_str(0, msg.head.sender, "redis read error"));
                return Ok(result);
            }
            let list = list.unwrap();
            let mut result: Vec<msg::Msg> = Vec::with_capacity(1);
            let json_str = serde_json::to_string(&list).unwrap();
            result.push(msg::Msg::msg_box(0, msg.head.sender, json_str));
            debug!("send box: {:?}", result);
            Ok(result)
        },
        _ => {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown msg type"))
        }
    }
}