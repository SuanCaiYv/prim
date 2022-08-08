use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;

pub async fn work(msg: &Msg, connection_map: MsgMap) -> Option<Msg> {
    match msg.head.typ {
        msg::Type::Text | msg::Type::Meme | msg::Type::Image | msg::Type::Video | msg::Type::Audio | msg::Type::File => {
            let receiver = msg.head.receiver;
            let mut sender: Option<&tokio::sync::oneshot::Sender<Msg>> = None;
            {
                let read_guard = connection_map.read().await;
                if let Some(s) = (*read_guard).get(&receiver) {
                    sender = Some(s);
                }
            }
            if let Some(sender) = sender {
                sender.send(msg.clone()).await.unwrap();
            }
            Some(msg.clone())
        }
        _ => {
            None
        }
    }
}