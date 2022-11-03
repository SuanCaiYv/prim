use super::{
    get_cluster_client_map, get_connection_map, ClusterClientMap, ClusterReceiver, ClusterSender,
    ConnectionMap,
};
use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use crate::util::my_id;
use ahash::AHashSet;
use common::entity::{Msg, NodeInfo, NodeStatus, Type};
use common::net::client::{Client, ClientConfigBuilder, ClientMultiConnection};
use common::util::jwt::simple_token;
use common::util::salt;
use common::Result;
use local_ip_address::list_afinet_netifas;
use std::net::IpAddr;
use std::sync::Arc;

pub(crate) struct ClientToBalancer {
    cluster_sender: ClusterSender,
}

impl ClientToBalancer {
    pub(crate) fn new(cluster_sender: ClusterSender) -> Self {
        Self { cluster_sender }
    }

    pub(crate) async fn registry_self(&self) -> Result<()> {
        let list = list_afinet_netifas().unwrap();
        let ip = list
            .iter()
            .filter(|(name, addr)| {
                if name == "en0" {
                    if let IpAddr::V6(_) = addr {
                        return true;
                    }
                }
                false
            })
            .map(|x| x.1)
            .collect::<Vec<IpAddr>>();
        let my_ip = ip[0].to_string();
        let my_address = format!("[{}]:{}", my_ip, CONFIG.server.address.port());
        let my_id = my_id();
        let addresses = &CONFIG.balancer.addresses;
        let index = my_id as usize % addresses.len();
        let balancer_address = addresses[index].clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_address(balancer_address)
            .with_domain(CONFIG.balancer.domain.clone())
            .with_cert(CONFIG.balancer.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams);
        let config = client_config.build().unwrap();
        let mut client = Client::new(config.clone());
        client.run().await?;
        let token_key = salt();
        let token = simple_token(token_key.as_bytes(), my_id as u64);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, my_id), token_key)
            .await?;
        let mut stream = client.rw_streams(my_id as u64, 0, token).await?;
        let node_info = NodeInfo {
            node_id: my_id,
            address: my_address.parse().expect("parse address error"),
            connection_id: 0,
            status: NodeStatus::DirectRegister,
        };
        let mut msg = Msg::raw_payload(&node_info.to_bytes());
        msg.set_type(Type::NodeRegister);
        stream.0.send(Arc::new(msg)).await?;
        loop {
            let msg = stream.1.recv().await;
            match msg {
                None => {
                    break;
                }
                Some(msg) => {
                    let _ = self.cluster_sender.send(msg).await;
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct ClusterClient {
    cluster_receiver: ClusterReceiver,
    cluster_client_map: ClusterClientMap,
    slot_set: AHashSet<u64>,
    connection_map: ConnectionMap,
}

impl ClusterClient {
    pub(crate) fn new(cluster_receiver: ClusterReceiver) -> Self {
        Self {
            cluster_receiver,
            cluster_client_map: get_cluster_client_map(),
            slot_set: AHashSet::new(),
            connection_map: get_connection_map(),
        }
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        loop {
            let msg = self.cluster_receiver.recv().await;
            match msg {
                None => {
                    break;
                }
                Some(msg) => {
                    match msg.typ() {
                        Type::NodeRegister => {
                            let node_info = NodeInfo::from(msg.payload());
                            self.new_node_online(&node_info).await?;
                        },
                        Type::NodeUnregister => {
                            let node_info = NodeInfo::from(msg.payload());
                            self.node_offline(&node_info).await?;
                        },
                        _ => {
                            continue;
                        }
                    };
                }
            };
        }
        Ok(())
    }

    pub(crate) async fn new_node_online(&mut self, node_info: &NodeInfo) -> Result<()> {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_address(node_info.address.clone())
            .with_domain(CONFIG.server.domain.clone())
            .with_cert(CONFIG.server.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams);
        let config = client_config.build().unwrap();
        // let multi_client = ClientMultiConnection::new(config).await?;
        let mut client = Client::new(config.clone());
        client.run().await?;
        let token_key = salt();
        let token = simple_token(token_key.as_bytes(), node_info.node_id as u64);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, node_info.node_id), token_key)
            .await?;
        let streams = client.rw_streams(node_info.node_id as u64, 0, token).await?;
        let res = self
            .cluster_client_map
            .insert(node_info.node_id, (streams.0, streams.1, client));
        Ok(())
    }

    pub(crate) async fn node_offline(&mut self, node_info: &NodeInfo) -> Result<()> {
        let res = self.cluster_client_map.remove(&node_info.node_id);
        if let Some((_, (_, _, mut client))) = res {
            client.wait_for_closed().await?;
        }
        Ok(())
    }
}
