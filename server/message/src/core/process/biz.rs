use tracing::{debug, error};
use crate::core::net;
use crate::entity::msg;
use crate::util::base;

pub async fn process(msg: &mut msg::Msg, c_map: &mut net::ConnectionMap, redis_ops: &mut net::RedisOps) -> std::io::Result<msg::Msg> {
    let mut key_incr = base::who_we_are(msg.head.sender, msg.head.receiver);
    key_incr.push_str("-seq_num");
    let seq_num: u64;
    {
        let mut lock = redis_ops.write().await;
        let seq_num_result = (*lock).atomic_increment(key_incr).await;
        if let Err(e) = seq_num_result {
            error!("redis read error: {}", e);
            return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
        }
        seq_num = seq_num_result.unwrap();
    }
    msg.head.seq_num = seq_num;
    let mut key_msg_channel = base::who_we_are(msg.head.sender, msg.head.receiver);
    key_msg_channel.push_str("-msg_channel");
    {
        let mut lock = redis_ops.write().await;
        if let Err(e) = (*lock).push_sort_queue(key_msg_channel, msg.clone(), seq_num as f64).await {
            error!("redis read error: {}", e);
            return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
        }
    }
    {
        let lock = c_map.read().await;
        let sender_option = (*lock).get(&(msg.head.receiver));
        if let Some(sender) = sender_option {
            // if let Err(e) =  {
            //     error!("send error: {}", e);
            //     return Ok(msg::Msg::err_msg(0, msg.head.sender, e.to_string()))
            // } else {
            //     debug!("send msg to {}", msg.head.receiver);
            // }
            sender.clone().send(msg.clone()).await.unwrap();
        }
    }
    Ok(msg.generate_ack(0))
}