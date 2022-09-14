use crate::core::net;
use crate::entity::msg;
use crate::util;

pub async fn process(msg: &msg::Msg, state_map: &net::StatusMap) -> std::io::Result<msg::Msg> {
    if let msg::Type::Heartbeat = msg.head.typ {
        state_map.insert(msg.head.sender, util::base::timestamp());
        Ok(msg::Msg::pong(msg.head.sender))
    } else {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "not heartbeat"));
    }
}