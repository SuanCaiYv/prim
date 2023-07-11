use std::{net::SocketAddr, sync::Arc, time::Duration};

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use lib::{
    cache::redis_ops::RedisOps,
    entity::{Msg, ReqwestMsg, ReqwestResourceID, Type},
    error::HandlerError,
    net::{client::ClientConfigBuilder, InnerStates, InnerStatesValue},
    util::{jwt::verify_token, timestamp},
    Result,
};
use lib_net_tokio::net::{client::ClientReqwestTcp, Handler, MsgSender, ReqwestOperatorManager};
use tracing::{debug, error};

use crate::{
    cache::USER_TOKEN,
    config::CONFIG,
    rpc::{get_rpc_client, node::RpcClient},
    service::MsgloggerClient,
};
use crate::{service::ClientConnectionMap, util::my_id};

use super::is_group_msg;

pub(crate) struct Auth {}

#[async_trait]
impl Handler for Auth {
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Auth != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let mut redis_ops;
        // to avoid borrow check conflict.
        {
            redis_ops = inner_states
                .get_mut("generic_map")
                .unwrap()
                .as_mut_generic_parameter_map()
                .unwrap()
                .get_parameter_mut::<RedisOps>()
                .unwrap()
                .clone();
        }
        let client_map = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<ClientConnectionMap>()
            .unwrap();
        let sender = inner_states
            .get("generic_map")
            .unwrap()
            .as_generic_parameter_map()
            .unwrap()
            .get_parameter::<MsgSender>()
            .unwrap();
        let token = String::from_utf8_lossy(msg.payload()).to_string();
        let key: String = redis_ops
            .get(&format!("{}{}", USER_TOKEN, msg.sender()))
            .await?;
        if let Err(e) = verify_token(&token, key.as_bytes(), msg.sender()) {
            error!("auth failed: {} {}", e, token);
            return Err(anyhow!(HandlerError::Auth(e.to_string())));
        }
        debug!("token verify succeed.");
        let mut res_msg = msg.generate_ack(my_id(), msg.timestamp());
        res_msg.set_type(Type::Auth);
        client_map.insert(msg.sender(), sender.clone());
        Ok(res_msg)
    }
}

pub(crate) struct Echo;

#[async_trait]
impl Handler for Echo {
    #[allow(unused)]
    async fn run(&self, msg: &mut Arc<Msg>, inner_states: &mut InnerStates) -> Result<Msg> {
        if Type::Echo != msg.typ() {
            return Err(anyhow!(HandlerError::NotMine));
        }
        if msg.receiver() == 0 {
            let mut res = (**msg).clone();
            res.set_receiver(msg.receiver());
            res.set_sender(0);
            res.set_timestamp(timestamp());
            Ok(res)
        } else {
            let client_timestamp = inner_states
                .get("client_timestamp")
                .unwrap()
                .as_num()
                .unwrap();
            Ok(msg.generate_ack(my_id(), client_timestamp))
        }
    }
}

pub(crate) struct PreProcess {
    seqnum_client: AHashMap<u32, ReqwestOperatorManager>,
}

impl PreProcess {
    pub(crate) async fn new() -> Self {
        let list = get_rpc_client().await.call_seqnum_all_node().await.unwrap();
        let address_list = list
            .1
            .into_iter()
            .map(|x| x.parse::<SocketAddr>().unwrap())
            .collect::<Vec<SocketAddr>>();
        let node_id_list = list.0;
        let mut map = AHashMap::new();
        for (i, address) in address_list.into_iter().enumerate() {
            let mut client_config = ClientConfigBuilder::default();
            client_config
                .with_remote_address(address)
                .with_ipv4_type(address.is_ipv4())
                .with_domain(CONFIG.server.domain.clone())
                .with_cert(CONFIG.server.cert.clone())
                .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
                .with_max_bi_streams(CONFIG.transport.max_bi_streams);
            let client_config = client_config.build().unwrap();
            let mut client = ClientReqwestTcp::new(client_config, Duration::from_millis(3000));
            let operator_manager = client.build().await.unwrap();
            Box::leak(Box::new(client));
            let node_id = node_id_list[i];
            map.insert(node_id, operator_manager);
        }
        Self {
            seqnum_client: AHashMap::new(),
        }
    }
}

#[async_trait]
impl Handler for PreProcess {
    async fn run(&self, msg: &mut Arc<Msg>, states: &mut InnerStates) -> Result<Msg> {
        let client_timestamp = msg.timestamp();
        let type_value = msg.typ().value();
        if type_value >= 32 && type_value < 64
            || type_value >= 64 && type_value < 96
            || type_value >= 128 && type_value < 160
        {
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
            let node_id = states
                .get("seqnum_node_select_map")
                .unwrap()
                .as_large_num_map()
                .unwrap()
                .get(&key)
                .unwrap();
            let operator_manager = self.seqnum_client.get(&(*node_id as u32)).unwrap();
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
            // let redis_ops = states
            //     .get_mut("generic_map")
            //     .unwrap()
            //     .as_mut_generic_parameter_map()
            //     .unwrap()
            //     .get_parameter_mut::<RedisOps>()
            //     .unwrap();
            // let seq_num;
            // if is_group_msg(msg.receiver()) {
            //     seq_num = redis_ops
            //         .atomic_increment(&format!(
            //             "{}{}",
            //             SEQ_NUM,
            //             who_we_are(msg.receiver(), msg.receiver())
            //         ))
            //         .await?;
            // } else {
            //     seq_num = redis_ops
            //         .atomic_increment(&format!(
            //             "{}{}",
            //             SEQ_NUM,
            //             who_we_are(msg.sender(), msg.receiver())
            //         ))
            //         .await?;
            // }
            match Arc::get_mut(msg) {
                Some(msg) => {
                    msg.set_seq_num(seqnum);
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
                .get_parameter_mut::<MsgloggerClient>()
                .unwrap();
            logger.log(msg.as_slice()).await?;
        }
        states.insert(
            "client_timestamp".to_owned(),
            InnerStatesValue::Num(client_timestamp),
        );
        let noop = Msg::noop();
        Ok(noop)
    }
}
