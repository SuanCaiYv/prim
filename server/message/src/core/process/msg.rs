use tracing::{debug, error};
use crate::core::net;
use crate::entity::msg;
use crate::util;

pub async fn process(msg: &mut msg::Msg, c_map: &mut net::ConnectionMap, redis_ops: &mut net::RedisOps) -> std::io::Result<msg::Msg> {
    // println!("{}", msg);
    match msg.head.typ {
        msg::Type::Text | msg::Type::File | msg::Type::Meme | msg::Type::Image | msg::Type::Audio | msg::Type::Video => {
            let client_timestamp = msg.head.timestamp;
            msg.head.timestamp = util::base::timestamp();
            super::common::sync_to_msg_channel(msg, redis_ops).await?;
            super::common::record_to_msg_box(msg, redis_ops).await?;
            // 唯一的错误就是连接被关闭，这属于正常结果，所以不应该报错
            let _ = super::common::try_send_msg_direct(msg, c_map).await;
            Ok(msg.generate_ack(client_timestamp))
        },
        _ => {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown msg type"))
        }
    }
}