pub(super) mod internal;
pub(super) mod logic;

use super::get_connection_map;
use common::entity::{Msg, NodeInfo, NodeStatus};
use common::net::client::Client;
use common::net::OuterReceiver;
use common::Result;
use common::{entity::Type, net::OuterSender};
use tracing::info;
use std::sync::Arc;

/// this function will observe the change of node cluster
/// and notify other nodes or balancers
pub(crate) async fn monitor(
    mut receiver: OuterReceiver,
    balancer_client_list: Vec<(OuterSender, OuterReceiver, Client)>,
) -> Result<()> {
    let connection_map = get_connection_map().0;
    loop {
        match receiver.recv().await {
            Some(msg) => match msg.typ() {
                Type::NodeRegister | Type::NodeUnregister => {
                    let mut node_info = NodeInfo::from(msg.payload());
                    for outer_sender in connection_map.iter() {
                        outer_sender.send(msg.clone()).await?;
                    }
                    match node_info.status {
                        NodeStatus::DirectRegister => {
                            node_info.status = NodeStatus::ClusterRegister;
                            let mut msg = Msg::raw_payload(&node_info.to_bytes());
                            msg.set_type(Type::NodeRegister);
                            let msg = Arc::new(msg);
                            for (sender, _, _) in balancer_client_list.iter() {
                                sender.send(msg.clone()).await?;
                            }
                        }
                        NodeStatus::DirectUnregister => {
                            node_info.status = NodeStatus::ClusterUnregister;
                            let mut msg = Msg::raw_payload(&node_info.to_bytes());
                            msg.set_type(Type::NodeUnregister);
                            let msg = Arc::new(msg);
                            for (sender, _, _) in balancer_client_list.iter() {
                                sender.send(msg.clone()).await?;
                            }
                        }
                        NodeStatus::ClusterRegister => {
                            info!("new node from other balancer registered");
                        }
                        NodeStatus::ClusterUnregister => {
                            info!("node from other balancer unregistered");
                        }
                    }
                }
                Type::BalancerRegister => {}
                _ => {}
            },
            None => {
                break;
            }
        }
    }
    Ok(())
}
