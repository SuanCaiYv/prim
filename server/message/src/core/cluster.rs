use super::{
    get_cluster_client_map, get_connection_map, ClusterClientMap, ClusterReceiver, ClusterSender,
    ConnectionMap,
};
use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use crate::util::my_id;
use ahash::AHashSet;
use common::entity::{Msg, NodeInfo, NodeStatus, Type};
use common::net::client::{Client, ClientConfigBuilder};
use common::net::CLUSTER_HASH_SIZE;
use common::util::jwt::simple_token;
use common::util::salt;
use common::Result;
use local_ip_address::list_afinet_netifas;
use std::net::IpAddr;
use std::sync::Arc;
use lazy_static::lazy_static;
use tokio::sync::RwLock;

lazy_static! {
    static ref CLIENT_ID_LIST: Arc<RwLock<Vec<u64>>> = Arc::new(RwLock::new(Vec::new()));
}

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
        let mut client = Client::new(config.clone(), my_id);
        client.run().await?;
        let token_key = salt();
        let token = simple_token(token_key.as_bytes(), my_id);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, my_id), token_key)
            .await?;
        let mut stream = client.rw_streams(my_id, token).await?;
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
                        Type::NodeRegister | Type::NodeUnregister => {
                            let node_info = NodeInfo::from(msg.payload());
                            if msg.typ() == Type::NodeRegister {
                                self.new_node_online(&node_info).await?;
                            } else {
                                self.node_offline(&node_info).await?;
                            }
                        }
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
        let mut client = Client::new(config.clone(), node_info.node_id);
        client.run().await?;
        let token_key = salt();
        let token = simple_token(token_key.as_bytes(), node_info.node_id);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, node_info.node_id), token_key)
            .await?;
        let streams = client.rw_streams(node_info.node_id, token).await?;
        let res = self
            .cluster_client_map
            .insert(node_info.node_id, (streams.0, streams.1, client));
        if res.is_none() {
            let mut list = CLIENT_ID_LIST.write().await;
            list.push(node_info.node_id);
            list.sort();
            self.cluster_scale().await?;
        }
        Ok(())
    }

    pub(crate) async fn node_offline(&mut self, node_info: &NodeInfo) -> Result<()> {
        let res = self.cluster_client_map.remove(&node_info.node_id);
        if let Some((_, (_, _, mut client))) = res {
            client.wait_for_closed().await?;
            let mut list = CLIENT_ID_LIST.write().await;
            let mut index = -1;
            for i in list.iter() {
                if *i == node_info.node_id {
                    index = *i as i64;
                    break;
                }
            }
            if index != -1 {
                list.remove(index as usize);
                list.sort();
                self.cluster_scale().await?;
            }
        }
        Ok(())
    }

    async fn cluster_scale(&mut self) -> Result<()> {
        let mut index = -1;
        let list = CLIENT_ID_LIST.read().await;
        for (i, v) in list.iter().enumerate() {
            if *v == my_id() {
                index = i as i64;
                break;
            }
        }
        if index != -1 {
            let a = CLUSTER_HASH_SIZE;
            let b = list.len() as u64;
            let m = a % b;
            let n = a / b;
            let index = index as u64;
            let mut start = index * n + m;
            let mut end = start + n;
            if index <= m - 1 {
                start = index * (n + 1);
                end = start + n;
            }
            let mut set = AHashSet::new();
            for i in start..=end {
                set.insert(i);
            }
            let mut change = self.slot_set.difference(&set).collect::<Vec<&u64>>();
            if self.slot_set.is_empty() {
                change = set.iter().collect::<Vec<&u64>>();
            }
            should_let_you_say_bye(&change, &self.connection_map).await?;
            self.slot_set = set;
        }
        Ok(())
    }
}

pub(crate) async fn which_node(receiver: u64) -> u64 {
    let list = CLIENT_ID_LIST.read().await;
    let a = CLUSTER_HASH_SIZE;
    let b = list.len() as u64;
    let mut c = receiver % a;
    let m = a % b;
    let n = a / b;
    let index = if m == 0 {
        let from = (n + 1) * m;
        if c <= from {
            c / (n + 1)
        } else {
            c -= from;
            m + c / n
        }
    } else {
        c / n
    };
    list[index as usize]
}

pub(self) async fn should_let_you_say_bye(
    list: &Vec<&u64>,
    connection_map: &ConnectionMap,
) -> Result<()> {
    let json_str = serde_json::to_string(list)?;
    // to avoid unnecessary copy of msg, we set receiver with the same value.
    let mut msg = Msg::text(0, 0, json_str);
    msg.set_type(Type::BeOffline);
    let msg = Arc::new(msg);
    for entry in connection_map.0.iter() {
        entry.value().send(msg.clone()).await?;
    }
    Ok(())
}
