use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;

pub async fn work(msg: &Msg, connection_map: MsgMap) -> Option<Msg> {
    match msg.head.typ {
        msg::Type::Ack | msg::Type::Sync | msg::Type::Offline | msg::Type::Auth => {
            Some(msg.clone())
        }
        _ => {
            None
        }
    }
}