pub(super) mod internal;
pub(super) mod logic;

use super::{get_connection_map, get_status_map};
use common::entity::{NodeInfo, NodeStatus};
use common::net::client::Client;
use common::net::OuterReceiver;
use common::Result;
use common::{entity::Type, net::OuterSender};

/// this function will observe the change of node cluster
/// and notify other nodes or balancers
pub(crate) async fn monitor(
    mut receiver: OuterReceiver,
    balancer_client_list: Vec<(OuterSender, OuterReceiver, Client)>,
) -> Result<()> {
    let status_map = get_status_map().0;
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
                            let msg = Msg::raw_payload(&node_info.to_bytes());
                            let msg = msg.update_type(Type::NodeRegister);
                            for (sender, _, _) in balancer_client_list.iter() {
                                sender.send(msg.clone()).await?;
                            }
                        }
                        NodeStatus::DirectUnregister => {
                            node_info.status = NodeStatus::ClusterUnregister;
                            let msg = Msg::raw_payload(&node_info.to_bytes());
                            let msg = msg.update_type(Type::NodeUnregister);
                            for (sender, _, _) in balancer_client_list.iter() {
                                sender.send(msg.clone()).await?;
                            }
                        }
                        NodeStatus::ClusterRegister => {}
                        NodeStatus::ClusterUnregister => {}
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
