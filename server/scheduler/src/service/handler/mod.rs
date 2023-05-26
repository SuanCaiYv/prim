pub(super) mod logic;
pub(super) mod message;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use tracing::error;

use lib::Result;

use crate::cluster::get_cluster_connection_map;
use crate::util::my_id;

use super::{
    get_client_connection_map, get_message_node_set, get_scheduler_node_set, get_server_info_map,
};

pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    mut timeout: MsgMpscReceiver,
    handler_list: &HandlerList,
    inner_states: &mut InnerStates,
) -> Result<()> {
    let client_map = get_client_connection_map();
    let server_info_map = get_server_info_map();
    let message_node_set = get_message_node_set();
    let scheduler_node_set = get_scheduler_node_set();
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    handler_parameters
        .generic_parameters
        .put_parameter(get_client_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(get_server_info_map());
    handler_parameters
        .generic_parameters
        .put_parameter(get_cluster_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(get_message_node_set());
    handler_parameters
        .generic_parameters
        .put_parameter(get_scheduler_node_set());
    handler_parameters
        .generic_parameters
        .put_parameter(sender.clone());
    let server_info;
    match receiver.recv().await {
        Some(auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let auth_handler = &handler_list[0];
            match auth_handler
                .run(auth_msg.clone(), &mut handler_parameters, inner_states)
                .await
            {
                Ok(res_msg) => {
                    server_info = ServerInfo::from(res_msg.payload());
                    sender.send(Arc::new(res_msg)).await?;
                }
                Err(e) => {
                    error!("auth failed: {}", e);
                    let err_msg = Msg::err_msg(my_id() as u64, auth_msg.sender(), 0, "auth failed");
                    sender.send(Arc::new(err_msg)).await?;
                    return Err(anyhow!("auth failed"));
                }
            }
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
                    error!("node [{}] crashed.", server_info.id);
                    break;
                }
            }
        }
    });
    loop {
        let msg = receiver.recv().await;
        match msg {
            Some(msg) => {
                let (msg, client_timestamp) = preprocessing(msg).await?;
                inner_states.insert(
                    "client_timestamp".to_string(),
                    InnerStatesValue::Num(client_timestamp),
                );
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
                client_map.remove(&server_info.id);
                server_info_map.remove(&server_info.id);
                if server_info.id >= MESSAGE_NODE_ID_BEGINNING
                    && server_info.id < SCHEDULER_NODE_ID_BEGINNING as u32
                {
                    msg.set_type(Type::MessageNodeUnregister);
                    message_node_set.remove(&server_info.id);
                } else if server_info.id >= SCHEDULER_NODE_ID_BEGINNING as u32 {
                    msg.set_type(Type::SchedulerNodeUnregister);
                    scheduler_node_set.remove(&server_info.id);
                } else {
                    return Err(anyhow!("invalid node id"));
                }
                msg.set_sender(server_info.id as u64);
                let msg = Arc::new(msg);
                call_handler_list(
                    &sender,
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

#[inline(always)]
pub(crate) async fn call_handler_list(
    sender: &MsgSender,
    msg: Arc<Msg>,
    handler_list: &HandlerList,
    handler_parameters: &mut HandlerParameters,
    inner_states: &mut InnerStates,
) -> Result<()> {
    for handler in handler_list.iter() {
        match handler
            .run(msg.clone(), handler_parameters, inner_states)
            .await
        {
            Ok(ok_msg) => {
                match ok_msg.typ() {
                    Type::Noop => {}
                    Type::Ack => {
                        sender.send(Arc::new(ok_msg)).await?;
                    }
                    _ => {
                        sender.send(Arc::new(ok_msg)).await?;
                        let client_timestamp = inner_states
                            .get("client_timestamp")
                            .unwrap()
                            .as_num()
                            .unwrap();
                        let mut ack_msg = msg.generate_ack(my_id(), client_timestamp);
                        ack_msg.set_sender(my_id() as u64);
                        ack_msg.set_receiver(msg.sender());
                        // todo()!
                        ack_msg.set_seq_num(0);
                        sender.send(Arc::new(ack_msg)).await?;
                    }
                }
                break;
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

#[inline(always)]
pub(crate) async fn preprocessing(mut msg: Arc<Msg>) -> Result<(Arc<Msg>, u64)> {
    let client_timestamp = msg.timestamp();
    let type_value = msg.typ().value();
    if type_value >= 32 && type_value < 64
        || type_value >= 64 && type_value < 96
        || type_value >= 128 && type_value < 160
    {
        match Arc::get_mut(&mut msg) {
            Some(msg) => msg.set_timestamp(timestamp()),
            None => {
                return Err(anyhow!("cannot get mutable reference of msg"));
            }
        };
    }
    Ok((msg, client_timestamp))
}
