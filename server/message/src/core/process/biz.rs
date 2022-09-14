use crate::core::net;
use crate::entity::msg;
use crate::util;

type Result = std::io::Result<msg::Msg>;

pub async fn process(msg: &mut msg::Msg, c_map: &net::ConnectionMap, redis_ops: &mut net::RedisOps) -> Result {
    match msg.head.typ {
        msg::Type::FriendRelationship => {
            let client_timestamp = msg.head.timestamp;
            msg.head.timestamp = util::base::timestamp();
            super::common::sync_to_msg_channel(msg, redis_ops).await?;
            super::common::record_to_msg_box(msg, redis_ops).await?;
            super::common::try_send_msg_direct(msg, c_map).await?;
            Ok(msg.generate_ack(client_timestamp))
        },
        _ => {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown msg type"))
        }
    }
}