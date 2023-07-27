use std::{net::SocketAddr, sync::Arc, time::Duration};

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use lib::{
    entity::{Msg, ReqwestMsg, ReqwestResourceID},
    error::HandlerError,
    net::{client::ClientConfigBuilder, InnerStates, InnerStatesValue},
    util::timestamp,
    Result,
};
use lib_net_tokio::net::{client::ClientReqwestTcp, Handler, ReqwestOperatorManager};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use tokio::sync::RwLock;
use tracing::error;

use crate::{
    config::config,
    rpc::{get_rpc_client, node::RpcClient},
    service::{get_mq_producer, get_seqnum_client_holder, handler::is_group_msg, Msglogger},
    util::my_id,
};

pub(crate) struct PreProcess {
    seqnum_client: Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>>,
}

impl PreProcess {
    pub(crate) fn new(
        seqnum_client_map: Arc<RwLock<AHashMap<u32, ReqwestOperatorManager>>>,
    ) -> Self {
        Self {
            seqnum_client: seqnum_client_map,
        }
    }
}

#[async_trait]
impl Handler for PreProcess {
    async fn run(&self, msg: &mut Arc<Msg>, states: &mut InnerStates) -> Result<Msg> {
        let client_timestamp = msg.timestamp();
        let type_value = msg.typ().value();
        if type_value >= 32 && type_value < 96 || type_value >= 128 && type_value < 160 {
            let key: u128 = if is_group_msg(msg.receiver()) {
                (msg.receiver() as u128) << 64 | msg.receiver() as u128
            } else {
                if msg.sender() < msg.receiver() {
                    (msg.sender() as u128) << 64 | msg.receiver() as u128
                } else {
                    (msg.receiver() as u128) << 64 | msg.sender() as u128
                }
            };
            if states.get("seqnum_node_select_map").is_none() {
                states.insert(
                    "seqnum_node_select_map".to_owned(),
                    InnerStatesValue::LargeNumMap(AHashMap::new()),
                );
            }
            if states
                .get("generic_map")
                .unwrap()
                .as_generic_parameter_map()
                .unwrap()
                .get_parameter::<RpcClient>()
                .is_none()
            {
                let rpc_client = get_rpc_client().await;
                states
                    .get_mut("generic_map")
                    .unwrap()
                    .as_mut_generic_parameter_map()
                    .unwrap()
                    .put_parameter(rpc_client);
            }
            if states
                .get("seqnum_node_select_map")
                .unwrap()
                .as_large_num_map()
                .unwrap()
                .get(&key)
                .is_none()
            {
                let rpc_client = states
                    .get_mut("generic_map")
                    .unwrap()
                    .as_mut_generic_parameter_map()
                    .unwrap()
                    .get_parameter_mut::<RpcClient>()
                    .unwrap();
                let node_id = match rpc_client.call_seqnum_node_user_select(key).await {
                    Ok(node_id) => node_id,
                    Err(e) => {
                        error!("call_seqnum_node_user_select failed: {}", e);
                        return Err(anyhow!(HandlerError::Other(
                            "call_seqnum_node_user_select failed".to_string()
                        )));
                    }
                };
                states
                    .get_mut("seqnum_node_select_map")
                    .unwrap()
                    .as_mut_large_num_map()
                    .unwrap()
                    .insert(key, node_id as u64);
            }
            let node_id = *states
                .get("seqnum_node_select_map")
                .unwrap()
                .as_large_num_map()
                .unwrap()
                .get(&key)
                .unwrap();
            let flag;
            {
                let map = self.seqnum_client.read().await;
                flag = map.get(&(node_id as u32)).is_none();
            }
            if flag {
                let rpc_client = states
                    .get_mut("generic_map")
                    .unwrap()
                    .as_mut_generic_parameter_map()
                    .unwrap()
                    .get_parameter_mut::<RpcClient>()
                    .unwrap();
                let address = match rpc_client.call_seqnum_node_address(node_id as u32).await {
                    Ok(address) => match address.parse::<SocketAddr>() {
                        Ok(address) => address,
                        Err(e) => {
                            error!("parse address failed: {}", e);
                            return Err(anyhow!(HandlerError::Other(
                                "parse address failed".to_string()
                            )));
                        }
                    },
                    Err(e) => {
                        error!("call_seqnum_node_address failed: {}", e);
                        return Err(anyhow!(HandlerError::Other(
                            "call_seqnum_node_address failed".to_string()
                        )));
                    }
                };
                let mut client_config = ClientConfigBuilder::default();
                client_config
                    .with_remote_address(address)
                    .with_ipv4_type(address.is_ipv4())
                    .with_domain(config().server.domain.clone())
                    .with_cert(config().server.cert.clone())
                    .with_keep_alive_interval(config().transport.keep_alive_interval)
                    .with_max_bi_streams(config().transport.max_bi_streams);
                let client_config = client_config.build().unwrap();
                let mut client = ClientReqwestTcp::new(client_config, Duration::from_millis(3000));
                let operator_manager = match client.build().await {
                    Ok(operator_manager) => operator_manager,
                    Err(e) => {
                        error!("build client failed: {}", e);
                        return Err(anyhow!(HandlerError::Other(
                            "build client failed".to_string()
                        )));
                    }
                };
                get_seqnum_client_holder()
                    .write()
                    .await
                    .insert(node_id as u32, client);
                let mut map = self.seqnum_client.write().await;
                map.insert(node_id as u32, operator_manager);
            }
            let map = self.seqnum_client.read().await;
            let operator_manager = map.get(&(node_id as u32)).unwrap();
            let mut data = [0u8; 16];
            BigEndian::write_u64(&mut data[0..8], (key >> 64) as u64);
            BigEndian::write_u64(&mut data[8..16], key as u64);
            let seqnum = match operator_manager
                .call(ReqwestMsg::with_resource_id_payload(
                    ReqwestResourceID::Seqnum,
                    &data,
                ))
                .await
            {
                Ok(resp) => BigEndian::read_u64(&resp.payload()[0..8]),
                Err(e) => {
                    error!("call seqnum failed: {}", e);
                    return Err(anyhow!(HandlerError::Other(
                        "call seqnum failed".to_string()
                    )));
                }
            };
            match Arc::get_mut(msg) {
                Some(msg) => {
                    msg.set_seqnum(seqnum);
                    msg.set_timestamp(timestamp());
                    // in case of client forgot set real sender.
                    if is_group_msg(msg.receiver()) && msg.extension_length() == 0 {
                        let bytes = msg.sender().to_string();
                        msg.0.extend_from_slice(bytes.as_bytes());
                    }
                }
                None => {
                    return Err(anyhow!("cannot get mutable reference of msg"));
                }
            };
            let logger = states
                .get_mut("generic_map")
                .unwrap()
                .as_mut_generic_parameter_map()
                .unwrap()
                .get_parameter_mut::<Msglogger>()
                .unwrap();
            logger.log(msg.clone()).await?;
        }
        states.insert(
            "client_timestamp".to_owned(),
            InnerStatesValue::Num(client_timestamp),
        );
        let noop = Msg::noop();
        Ok(noop)
    }
}

pub(crate) struct MQPusher {
    mq_producer: FutureProducer,
    topic_name: String,
}

impl MQPusher {
    pub(crate) fn new() -> Self {
        Self {
            mq_producer: get_mq_producer(),
            topic_name: format!("msg-{:06}", my_id()),
        }
    }
}

#[async_trait]
impl Handler for MQPusher {
    async fn run(&self, msg: &mut Arc<Msg>, _states: &mut InnerStates) -> Result<Msg> {
        let type_value = msg.typ().value();
        if type_value >= 32 && type_value < 96 || type_value >= 128 && type_value < 160 {
        } else {
            return Ok(Msg::noop());
        }
        let mut ok = false;
        for _ in 0..5 {
            match self
                .mq_producer
                .send(
                    FutureRecord::to(&self.topic_name)
                        .key(msg.seqnum().to_string().as_bytes())
                        .timestamp(timestamp() as i64)
                        .payload(msg.as_slice()),
                    Timeout::After(Duration::from_millis(3000)),
                )
                .await
            {
                Ok(_) => {
                    ok = true;
                    break;
                }
                Err(e) => {
                    error!("send message to kafka failed: {}", e.0.to_string());
                    continue;
                }
            }
        }
        if !ok {
            error!("send message to kafka failed, may be to busy: {:?}", msg.0);
        }
        Ok(Msg::noop())
    }
}
