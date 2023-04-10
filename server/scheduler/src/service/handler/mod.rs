pub(super) mod message;
pub(super) mod recorder;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use lib::entity::{Msg, ServerInfo, ServerStatus, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, HandlerList};
use lib::net::{MsgMpscReceiver, MsgMpscSender, MsgSender};
use lib::{net::server::HandlerParameters, Result, SCHEDULER_NODE_ID_BEGINNING};
use lib::{MESSAGE_NODE_ID_BEGINNING, RECORDER_NODE_ID_BEGINNING};
use tracing::error;

use crate::cluster::get_cluster_connection_map;
use crate::config::CONFIG;
use crate::util::my_id;

use super::{
    get_client_connection_map, get_message_node_set, get_recorder_node_set, get_scheduler_node_set,
    get_server_info_map,
};

pub(super) async fn handler_func(
    sender: MsgMpscSender,
    mut receiver: MsgMpscReceiver,
    mut timeout: MsgMpscReceiver,
    handler_list: &HandlerList<()>,
    inner_states: &mut InnerStates<()>,
) -> Result<()> {
    let client_map = get_client_connection_map();
    let server_info_map = get_server_info_map();
    let message_node_set = get_message_node_set();
    let scheduler_node_set = get_scheduler_node_set();
    let recorder_node_set = get_recorder_node_set();
    let server_info = match receiver.recv().await {
        Some(auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let server_info = ServerInfo::from(auth_msg.payload());
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
                typ: server_info.typ,
                load: None,
            };
            let mut res_msg = Msg::raw_payload(&res_server_info.to_bytes());
            res_msg.set_type(Type::Auth);
            res_msg.set_sender(my_id() as u64);
            res_msg.set_receiver(server_info.id as u64);
            sender.send(Arc::new(res_msg)).await?;
            client_map.0.insert(server_info.id, sender.clone());
            server_info_map.0.insert(server_info.id, server_info);
            if server_info.id >= MESSAGE_NODE_ID_BEGINNING
                && server_info.id < SCHEDULER_NODE_ID_BEGINNING
            {
                message_node_set.0.insert(server_info.id);
            } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING
                && server_info.id < RECORDER_NODE_ID_BEGINNING
            {
                scheduler_node_set.0.insert(server_info.id);
            } else if server_info.id >= RECORDER_NODE_ID_BEGINNING {
                recorder_node_set.0.insert(server_info.id);
            }
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
            let failed_msg = timeout.recv().await;
            match failed_msg {
                Some(failed_msg) => {
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
                                if let Err(e) = io_sender.send(failed_msg).await {
                                    error!("retry failed send msg. error: {}", e);
                                    break;
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
    handler_parameters
        .generic_parameters
        .put_parameter(client_map);
    handler_parameters
        .generic_parameters
        .put_parameter(server_info_map);
    handler_parameters
        .generic_parameters
        .put_parameter(get_cluster_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(message_node_set);
    handler_parameters
        .generic_parameters
        .put_parameter(scheduler_node_set);
    handler_parameters
        .generic_parameters
        .put_parameter(recorder_node_set);
    let sender = MsgSender::Server(sender);
    loop {
        let msg = receiver.recv().await;
        match msg {
            Some(msg) => {
                call_handler_list(
                    &sender,
                    &mut receiver,
                    msg,
                    &handler_list,
                    &mut handler_parameters,
                    inner_states,
                )
                .await?;
            }
            None => {
                error!("io receiver closed");
                let res_server_info = ServerInfo {
                    id: server_info.id,
                    service_address: server_info.service_address,
                    cluster_address: server_info.cluster_address,
                    connection_id: 0,
                    status: ServerStatus::Offline,
                    typ: server_info.typ,
                    load: None,
                };
                let mut msg = Msg::raw_payload(&res_server_info.to_bytes());
                if server_info.id >= 1 && server_info.id < SCHEDULER_NODE_ID_BEGINNING as u32 {
                    msg.set_type(Type::MessageNodeUnregister)
                } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING as u32
                    && server_info.id < RECORDER_NODE_ID_BEGINNING as u32
                {
                    msg.set_type(Type::SchedulerNodeUnregister)
                } else if server_info.id >= RECORDER_NODE_ID_BEGINNING as u32 {
                    msg.set_type(Type::RecorderNodeUnregister)
                } else {
                    return Err(anyhow!("invalid node id"));
                }
                msg.set_sender(server_info.id as u64);
                let msg = Arc::new(msg);
                call_handler_list(
                    &sender,
                    &mut receiver,
                    msg,
                    &handler_list,
                    &mut handler_parameters,
                    inner_states,
                )
                .await?;
                break;
            }
        }
    }
    Ok(())
}

async fn call_handler_list(
    sender: &MsgSender,
    _receiver: &mut MsgMpscReceiver,
    msg: Arc<Msg>,
    handler_list: &HandlerList<()>,
    handler_parameters: &mut HandlerParameters,
    inner_states: &mut InnerStates<()>,
) -> Result<()> {
    for handler in handler_list.iter() {
        match handler
            .run(msg.clone(), handler_parameters, inner_states)
            .await
        {
            Ok(ok_msg) => {
                match ok_msg.typ() {
                    Type::Noop => {
                        break;
                    }
                    Type::Ack => {
                        sender.send(Arc::new(ok_msg)).await?;
                    }
                    _ => {
                        sender.send(Arc::new(ok_msg)).await?;
                        let mut ack_msg = msg.generate_ack(my_id());
                        ack_msg.set_sender(my_id() as u64);
                        ack_msg.set_receiver(msg.sender());
                        // todo()!
                        ack_msg.set_seq_num(0);
                        sender.send(Arc::new(ack_msg)).await?;
                    }
                }
            }
            Err(e) => {
                match e.downcast::<HandlerError>() {
                    Ok(handler_err) => match handler_err {
                        HandlerError::NotMine => {
                            continue;
                        }
                        HandlerError::Auth { .. } => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), "auth failed");
                            sender.send(Arc::new(res_msg)).await?;
                        }
                        HandlerError::Parse(cause) => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), &cause);
                            sender.send(Arc::new(res_msg)).await?;
                        }
                    },
                    Err(e) => {
                        error!("unhandled error: {}", e);
                        let res_msg =
                            Msg::err_msg(my_id() as u64, msg.sender(), my_id(), "unhandled error");
                        sender.send(Arc::new(res_msg)).await?;
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}
