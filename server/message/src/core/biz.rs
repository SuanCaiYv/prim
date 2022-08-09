use tracing::{debug, error, info, warn};
use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;
use crate::persistence::redis_ops::RedisOps;

pub async fn work(msg: &mut Msg, connection_map: &mut MsgMap, redis_ops: &mut RedisOps) -> Option<Msg> {
    match msg.head.typ {
        msg::Type::Text | msg::Type::Meme | msg::Type::Image | msg::Type::Video | msg::Type::Audio | msg::Type::File => {
            // 实际发送
            let mut sender: Option<tokio::sync::mpsc::Sender<Msg>> = None;
            {
                let read_guard = connection_map.read().await;
                let sender0 = (*read_guard).get(&msg.head.receiver);
                if let Some(sender0) = sender0 {
                    sender = Some(sender0.clone());
                }
            }
            if let Some(sender) = sender {
                sender.send(msg.clone()).await.unwrap();
            }
            let mut incr_key = util::base::who_we_are(msg.head.sender, msg.head.receiver);
            incr_key.push_str("-seq_num");
            let seq_num = redis_ops.atomic_increment(incr_key).await;
            if let Err(e) = seq_num {
                error!("redis read error: {}", e);
                return Some(Msg::err_msg_str(0, msg.head.sender, "redis read error"));
            }
            let seq_num = seq_num.unwrap();
            msg.head.seq_num = seq_num;
            let mut msg_channel_key = util::base::who_we_are(msg.head.sender, msg.head.receiver);
            msg_channel_key.push_str("-msg_channel");
            let push_result = redis_ops.push_sort_queue(msg_channel_key, msg.clone(), seq_num as f64).await;
            if let Err(e) = push_result {
                error!("redis read error: {}", e);
                return Some(Msg::err_msg_str(0, msg.head.sender, "redis read error"));
            }
            Some(msg.generate_ack(0))
        }
        _ => {
            None
        }
    }
}