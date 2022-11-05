use super::{get_cluster_client_map, ClusterClientMap, ClusterReceiver, ClusterSender};
use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use crate::util::my_id;
use common::entity::{Msg, NodeInfo, NodeStatus, Type};
use common::net::client::{
    Client, ClientConfigBuilder, ClientMultiConnection, ClientSubConnectionConfig,
};
use common::util::jwt::simple_token;
use common::util::{my_ip, salt, default_bind_ip};
use common::Result;
use std::sync::Arc;
use tracing::error;
use crate::core::{ConnectionMap, get_connection_map};

pub(crate) struct ClientToBalancer {
    cluster_sender: ClusterSender,
}

impl ClientToBalancer {
    pub(crate) fn new(cluster_sender: ClusterSender) -> Self {
        Self { cluster_sender }
    }

    pub(crate) async fn registry_self(&self) -> Result<()> {
        let my_address;
        let m_ip = my_ip();
        if m_ip.len() > 15 {
            my_address = format!("[{}]:{}", my_ip(), CONFIG.server.address.port());
        } else {
            my_address = format!("{}:{}", my_ip(), CONFIG.server.address.port());
        }
        let my_id = my_id();
        let addresses = &CONFIG.balancer.addresses;
        let index = my_id as usize % addresses.len();
        let balancer_address = addresses[index].clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(balancer_address)
            .with_domain(CONFIG.balancer.domain.clone())
            .with_cert(CONFIG.balancer.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_task_channel_size(CONFIG.performance.max_task_channel_size)
            .with_max_io_channel_size(CONFIG.performance.max_io_channel_size);
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
        msg.set_sender(my_id as u64);
        msg.set_receiver(0);
        msg.set_sender_node(my_id);
        msg.set_receiver_node(0);
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
    multi_client: ClientMultiConnection,
    connection_map: ConnectionMap,
}

impl ClusterClient {
    pub(crate) async fn new(cluster_receiver: ClusterReceiver) -> Result<Self> {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(default_bind_ip()) // note: this address is not used
            .with_domain(CONFIG.server.domain.clone())
            .with_cert(CONFIG.server.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_task_channel_size(CONFIG.performance.max_task_channel_size)
            .with_max_io_channel_size(CONFIG.performance.max_io_channel_size);
        let config = client_config.build().unwrap();
        let multi_client = ClientMultiConnection::new(config).await?;
        Ok(Self {
            cluster_receiver,
            cluster_client_map: get_cluster_client_map(),
            multi_client,
            connection_map: get_connection_map(),
        })
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
                            if node_info.node_id != my_id() {
                                let res = self.new_node_online(&node_info).await;
                                if let Err(e) = res {
                                    error!("new node online error: {}", e);
                                }
                            }
                        }
                        Type::NodeUnregister => {
                            let node_info = NodeInfo::from(msg.payload());
                            if node_info.node_id != my_id() {
                                let res = self.node_offline(&node_info).await;
                                if res.is_err() {
                                    error!("node_offline error: {:?}", res);
                                }
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
        let mut sub_connection = self
            .multi_client
            .new_connection(ClientSubConnectionConfig {
                remote_address: node_info.address,
                domain: CONFIG.server.domain.clone(),
                opend_bi_streams_number: 3,
                opend_uni_streams_number: 3,
            })
            .await?;
        let token_key = salt();
        let token = simple_token(token_key.as_bytes(), my_id() as u64);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, my_id() as u64), token_key)
            .await?;
        let streams = sub_connection
            .operation_channel(my_id() as u64, 0, token)
            .await?;
        let text = format!("hello new peer.");
        let msg = Msg::text(
            my_id() as u64,
            node_info.node_id as u64,
            my_id(),
            node_info.node_id,
            text,
        );
        streams.0.send(Arc::new(msg)).await?;
        let _ = self
            .cluster_client_map
            .insert(node_info.node_id, (streams.0, streams.1, sub_connection));
        Ok(())
    }

    pub(crate) async fn node_offline(&mut self, node_info: &NodeInfo) -> Result<()> {
        error!("peer[{}] dead", node_info.node_id);
        let res = self.connection_map.0.get(&(node_info.node_id as u64));
        if let Some(connection) = res {
            connection.close();
        }
        let res = self.cluster_client_map.remove(&node_info.node_id);
        if let Some((_, (_, _, mut client))) = res {
            client.wait_for_closed().await?;
        }
        Ok(())
    }
}
