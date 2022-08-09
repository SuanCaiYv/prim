use crate::entity::msg;
use crate::{Msg, net, util};

pub async fn work(msg: &Msg, state_map: &mut net::StatusMap) -> Option<Msg> {
    if let msg::Type::Heartbeat = msg.head.typ {
        {
            let mut write_guard = state_map.write().await;
            (*write_guard).insert(msg.head.sender, util::base::timestamp());
        }
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