use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;

pub async fn work(msg: &Msg, connection_map: MsgMap) -> Option<Msg> {
    match msg.head.typ {
        msg::Type::Text | msg::Type::Meme | msg::Type::Image | msg::Type::Video | msg::Type::Audio | msg::Type::File => {
            Some(msg.clone())
        }
        _ => {
            None
        }
    }
}