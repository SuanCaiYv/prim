pub(super) mod internal;
pub(super) mod logic;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use lib::entity::{Msg, Type};
use lib::net::server::{GenericParameterMap, HandlerList, InnerStates};
use lib::net::MsgSender;
use lib::{
    net::{server::HandlerParameters, MsgMpscReceiver},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::cluster::get_cluster_connection_map;
use crate::service::get_client_connection_map;
use crate::service::handler::{call_handler_list, IOTaskSender};
use crate::util::my_id;

pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    mut timeout_receiver: MsgMpscReceiver,
    io_task_sender: IOTaskSender,
    handler_list: &HandlerList,
    inner_states: &mut InnerStates,
) -> Result<()> {
    // todo integrate with service
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    handler_parameters
        .generic_parameters
        .put_parameter(get_redis_ops().await);
    handler_parameters
        .generic_parameters
        .put_parameter(get_client_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(io_task_sender);
    handler_parameters
        .generic_parameters
        .put_parameter(get_cluster_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(sender.clone());
    let io_sender = sender.clone();
    let scheduler_id;
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
                    sender.send(Arc::new(res_msg)).await?;
                    scheduler_id = auth_msg.sender() as u32;
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
    tokio::spawn(async move {
        let mut retry_count = AHashMap::new();
        loop {
            let failed_msg = timeout_receiver.recv().await;
            match failed_msg {
                Some(failed_msg) => {
                    let key = failed_msg.timestamp() % 4000;
                    match retry_count.get(&key) {
                        Some(count) => {
                            if *count == 0 {
                                // todo impact error should be handled manually.
                                error!(
                                    "retry too many times, peer may busy or crashed. msg: {}",
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
                    error!("scheduler[{}] crashed.", scheduler_id);
                    break;
                }
            }
        }
    });
    loop {
        let msg = receiver.recv().await;
        match msg {
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
                error!("io receiver closed");
                break;
            }
        }
    }
    Ok(())
}
