pub(super) mod internal;
pub(super) mod logic;

use common::entity::Type;
use common::net::OuterReceiver;
use common::Result;

use super::{get_status_map, get_connection_map};

pub(crate) async fn monitor(mut receiver: OuterReceiver) -> Result<()> {
    let status_map = get_status_map().0;
    let connection_map = get_connection_map().0;
    loop {
        match receiver.recv().await {
            Some(msg) => {
                match msg.typ() {
                    Type::Register | Type::Unregister => {
                        for node in status_map.iter() {
                            let node = node.value();
                            let outer_sender = connection_map.get(&node.node_id);
                            if outer_sender.is_none() {
                                continue;
                            }
                            let outer_sender = outer_sender.unwrap();
                            outer_sender.send(msg.clone()).await?;
                        }
                    },
                    _ => {},
                }
            }
            None => {
                break;
            }
        }
    }
    Ok(())
}