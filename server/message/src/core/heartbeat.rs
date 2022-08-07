use crate::entity::msg;
use crate::{Msg, util};

pub async fn work(msg: &Msg) -> Option<Msg> {
    if let msg::Type::Heartbeat = msg.head.typ {
        let head = msg::Head {
            length: 4,
            typ: msg::Type::Heartbeat,
            sender: 0,
            receiver: msg.head.sender,
            timestamp: util::base::timestamp(),
            seq_num: 0,
            version: 0
        };
        Some(Msg {
            head,
            payload: Vec::from("pong"),
        })
    } else {
        return None;
    }
}