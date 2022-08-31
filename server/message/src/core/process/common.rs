use tracing::{debug, error, warn};
use crate::core::net;
use crate::entity::msg;
use crate::util;

type Result = std::io::Result<()>;

pub async fn sync_to_msg_channel(msg: &mut msg::Msg, redis_ops: &mut net::RedisOps) -> Result {
    let mut key_incr = util::base::who_we_are(msg.head.sender, msg.head.receiver);
    key_incr.push_str("-seq_num");
    let seq_num: u64;
    let seq_num_result = redis_ops.atomic_increment(key_incr).await;
    if let Err(e) = seq_num_result {
        error!("redis read/write error: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    seq_num = seq_num_result.unwrap();
    msg.head.seq_num = seq_num;

    let mut key_msg_channel = util::base::who_we_are(msg.head.sender, msg.head.receiver);
    key_msg_channel.push_str("-msg_channel");
    debug!("sync msg: {} to redis {}", msg, key_msg_channel);
    if let Err(e) = redis_ops.push_sort_queue(key_msg_channel, msg.clone(), seq_num as f64).await {
        error!("redis write error: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    Ok(())
}

pub async fn record_to_msg_box(msg: &msg::Msg, redis_ops: &mut net::RedisOps) -> Result {
    let msg_box_channel = format!("{}-msg_box", msg.head.receiver);
    debug!("send msg: {} to box {}", msg, msg_box_channel);
    if let Err(e) = redis_ops.push_sort_queue(msg_box_channel, msg.head.sender, util::base::timestamp() as f64).await {
        error!("redis write error: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    Ok(())
}

pub async fn try_send_msg_direct(msg: &msg::Msg, c_map: &mut net::ConnectionMap) -> Result {
    let lock = c_map.read().await;
    let sender_option = (*lock).get(&(msg.head.receiver));
    if let Some(sender) = sender_option {
        if let Err(e) = sender.send(msg.clone()).await {
            warn!("send directly error: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        } else {
            debug!("send msg to {}", msg.head.receiver);
        }
    }
    Ok(())
}