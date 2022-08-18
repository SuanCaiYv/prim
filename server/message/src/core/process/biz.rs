use tracing::{debug, error};
use crate::core::net;
use crate::entity::msg;
use crate::util;

pub async fn process(msg: &mut msg::Msg, c_map: &mut net::ConnectionMap, redis_ops: &mut net::RedisOps) -> std::io::Result<msg::Msg> {
    let client_timestamp = msg.head.timestamp;
    msg.head.timestamp = util::base::timestamp();
    let mut key_incr = util::base::who_we_are(msg.head.sender, msg.head.receiver);
    key_incr.push_str("-seq_num");
    let seq_num: u64;
    let seq_num_result = redis_ops.atomic_increment(key_incr).await;
    if let Err(e) = seq_num_result {
        error!("redis read error: {}", e);
        return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
    }
    seq_num = seq_num_result.unwrap();
    msg.head.seq_num = seq_num;
    let mut key_msg_channel = util::base::who_we_are(msg.head.sender, msg.head.receiver);
    key_msg_channel.push_str("-msg_channel");
    if let Err(e) = redis_ops.push_sort_queue(key_msg_channel, msg.clone(), seq_num as f64).await {
        error!("redis read error: {}", e);
        return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
    }
    {
        let lock = c_map.read().await;
        let sender_option = (*lock).get(&(msg.head.receiver));
        if let Some(sender) = sender_option {
            if let Err(e) = sender.send(msg.clone()).await {
                error!("send error: {}", e);
                return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
            } else {
                debug!("send msg to {}", msg.head.receiver);
            }
        }
    }
    Ok(msg.generate_ack(client_timestamp))
}