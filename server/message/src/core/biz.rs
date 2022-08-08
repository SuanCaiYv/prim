use crate::{Msg, util};
use crate::entity::msg;
use crate::net::MsgMap;

pub async fn work(msg: &Msg, connection_map: MsgMap) -> Option<Msg> {
    match msg.head.typ {
        msg::Type::Text | msg::Type::Meme | msg::Type::Image | msg::Type::Video | msg::Type::Audio | msg::Type::File => {
            let receiver = msg.head.receiver;
            let mut sender: Option<tokio::sync::mpsc::Sender<Msg>> = None;
            {
                let read_guard = connection_map.read().await;
                let sender0 = (*read_guard).get(&receiver);
                if let Some(sender0) = sender0 {
                    sender = Some(sender0.clone());
                }
            }
            if let Some(sender) = sender {
                // 这个方法可能会阻塞
                sender.send(msg.clone()).await.unwrap();
            }
            Some(msg.clone())
        }
        _ => {
            None
        }
    }
}