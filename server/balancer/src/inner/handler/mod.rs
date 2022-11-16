pub(super) mod internal;
pub(super) mod logic;

use std::sync::Arc;
use common::entity::Msg;
use tracing::{error, info, warn};

use super::get_node_client_map;
use common::entity::Type;
use common::net::OuterReceiver;
use common::Result;
use crate::inner::get_status_map;

/// this function will observe the change of node cluster
/// and notify other nodes or balancers
pub(crate) async fn io_tasks(mut receiver: OuterReceiver) -> Result<()> {
    let node_client_map = get_node_client_map().0;
    let status_map = get_status_map().0;
    loop {
        match receiver.recv().await {
            Some(msg) => {
                match msg.typ() {
                    Type::NodeRegister => {
                        let node_info = NodeInfo::from(msg.payload());
                        info!("new node online: {}", node_info);
                        let mut msg = (*msg).clone();
                        msg.set_sender(0);
                        msg.set_receiver(0);
                        msg.set_sender_node(0);
                        msg.set_receiver_node(0);
                        let msg = Arc::new(msg);
                        for sender in node_client_map.iter() {
                            sender.send(msg.clone()).await?;
                        }
                        let new_peer_sender = node_client_map.get(&node_info.node_id);
                        if let Some(new_peer_sender) = new_peer_sender {
                            for info in status_map.iter() {
                                let mut msg = Msg::raw_payload(&info.to_bytes());
                                msg.set_type(Type::NodeRegister);
                                msg.set_sender(0);
                                msg.set_receiver(0);
                                msg.set_sender_node(0);
                                msg.set_receiver_node(0);
                                let msg = Arc::new(msg);
                                let res = new_peer_sender.send(msg).await;
                                if res.is_err() {
                                    error!("send new peer msg error: {}", res.err().unwrap());
                                }
                            }
                        }
                    }
                    Type::NodeUnregister => {
                        let node_info = NodeInfo::from(msg.payload());
                        warn!("node {} is offline.", node_info);
                        let mut msg = (*msg).clone();
                        msg.set_sender(0);
                        msg.set_receiver(0);
                        msg.set_sender_node(0);
                        msg.set_receiver_node(0);
                        let msg = Arc::new(msg);
                        for sender in node_client_map.iter() {
                            sender.send(msg.clone()).await?;
                        }
                    }
                    Type::BalancerRegister => {}
                    _ => {}
                }
            }
            None => {
                error!("monitor receiver is closed.");
                break;
            }
        }
    }
    Ok(())
}
