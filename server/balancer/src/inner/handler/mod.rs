pub(super) mod internal;
pub(super) mod logic;

use std::sync::Arc;
use tracing::{error, info, warn};

use super::get_node_client_map;
use super::get_status_map;
use common::entity::NodeInfo;
use common::entity::Type;
use common::net::OuterReceiver;
use common::Result;

/// this function will observe the change of node cluster
/// and notify other nodes or balancers
pub(crate) async fn monitor(mut receiver: OuterReceiver) -> Result<()> {
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
                    }
                    Type::NodeUnregister => {
                        let node_info = NodeInfo::from(msg.payload());
                        warn!("node {} is offline.", node_info);
                        let mut msg = (*msg).clone();
                        msg.set_sender(0);
                        msg.set_receiver(0);
                        msg.set_sender_node(0);
                        msg.set_receiver_node(0);
                        status_map.remove(&node_info.node_id);
                        node_client_map.remove(&node_info.node_id);
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
