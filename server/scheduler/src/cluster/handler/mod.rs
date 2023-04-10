mod logic;
pub(crate) mod message;

use std::sync::Arc;

use lib::entity::{Msg, ServerInfo, ServerStatus, ServerType, Type};
use lib::net::server::{GenericParameterMap, HandlerList, InnerStates};
use lib::{
    net::{server::HandlerParameters, MsgMpscReceiver},
    Result,
};

use ahash::AHashMap;
use anyhow::anyhow;
use tracing::{debug, error, info};

use crate::config::CONFIG;
use crate::service::handler::call_handler_list;
use crate::util::my_id;

use super::{get_cluster_connection_map, get_cluster_connection_set, MsgSender};

pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    mut timeout: MsgMpscReceiver,
    handler_list: &HandlerList,
    inner_states: &mut InnerStates,
) -> Result<()> {
    let cluster_set = get_cluster_connection_set();
    let cluster_map = get_cluster_connection_map().0;
    let server_info = match receiver.recv().await {
        Some(auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let server_info = ServerInfo::from(auth_msg.payload());
            info!("cluster server {} connected", server_info.id);
            let mut service_address = CONFIG.server.service_address;
            service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
            let mut cluster_address = CONFIG.server.cluster_address;
            cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
            let res_server_info = ServerInfo {
                id: my_id(),
                service_address,
                cluster_address: Some(cluster_address),
                connection_id: 0,
                status: ServerStatus::Normal,
                typ: ServerType::SchedulerCluster,
                load: None,
            };
            let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
            res_msg.set_type(Type::Auth);
            res_msg.set_sender(my_id() as u64);
            res_msg.set_receiver(server_info.id as u64);
            sender.send(Arc::new(res_msg)).await?;
            cluster_set.insert(server_info.cluster_address.unwrap());
            cluster_map.insert(server_info.id, sender.clone());
            debug!("start handler function of server.");
            server_info
        }
        None => {
            error!("cannot receive auth message");
            return Err(anyhow!("cannot receive auth message"));
        }
    };
    let io_sender = sender.clone();
    tokio::spawn(async move {
        let mut retry_count = AHashMap::new();
        loop {
            match timeout.recv().await {
                Some(failed_msg) => {
                    // todo retry recorder optimization
                    let key = failed_msg.timestamp() % 4000;
                    match retry_count.get(&key) {
                        Some(count) => {
                            if *count == 0 {
                                error!(
                                    "retry too many times, peer may busy or dead. msg: {}",
                                    failed_msg
                                );
                            } else {
                                retry_count.insert(key, *count - 1);
                                match io_sender {
                                    MsgSender::Client(ref sender) => {
                                        if let Err(e) = sender.send(failed_msg).await {
                                            error!("retry failed send msg. error: {}", e);
                                            break;
                                        }
                                    }
                                    MsgSender::Server(ref sender) => {
                                        if let Err(e) = sender.send(failed_msg).await {
                                            error!("retry failed send msg. error: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            retry_count.insert(key, 4);
                        }
                    }
                }
                None => {
                    error!("timeout receiver closed");
                    break;
                }
            }
        }
    });
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    loop {
        match receiver.recv().await {
            Some(msg) => {
                call_handler_list(
                    &sender,
                    msg,
                    &handler_list,
                    &mut handler_parameters,
                    inner_states,
                )
                .await?;
            }
            None => {
                error!("scheduler[{}] node crash", server_info.id);
                break;
            }
        }
    }
    Ok(())
}
