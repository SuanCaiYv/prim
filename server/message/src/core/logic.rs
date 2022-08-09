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

pub async fn work(msg: &Msg, connection_map: MsgMap, redis_ops: &mut RedisOps) -> Option<Vec<Msg>> {
    match msg.head.typ {
        msg::Type::Sync => {
            let params: serde_json::Result<SyncStruct> = serde_json::from_slice(msg.payload.as_slice());
            if let Err(e) = params {
                error!("parse params failed: {}", e);
                vec[0] = Msg::err_msg_str(0, msg.head.sender, "parse params failed");
                return Some(vec);
            }
            let params = params.unwrap();
            let mut key = util::base::who_we_are(msg.head.sender, msg.head.receiver);
            key.push_str("-msg_channel");
            let mut list: Vec<Msg> = Vec::new();
            list.push(Msg::default());
            if params.s == 0 {
                let result: RedisResult<Vec<Msg>> = redis_ops.peek_sort_queue_more(key, 0, 20).await;
                if let Err(e) = result {
                    error!("redis read error: {}", e);
                    vec[0] = Msg::err_msg_str(0, msg.head.sender, "redis read error");
                    return Some(vec);
                }
                list.append(&mut result.unwrap());
            } else {
                let result: RedisResult<Vec<Msg>> = redis_ops.peek_sort_queue_more(key, 0, 20).await;
            }
            Some(Vec::new())
        }
        msg::Type::Auth => {
            Some(Vec::new())
        }
        _ => {
            None
        }
    }
}