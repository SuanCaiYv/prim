pub(super) mod internal;

use ahash::AHashMap;
use lib::entity::ServerInfo;
use lib::net::server::{GenericParameterMap, HandlerList};
use lib::net::MsgSender;
use lib::{
    net::{server::HandlerParameters, MsgMpmcSender, MsgMpscReceiver},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::cluster::get_cluster_connection_map;
use crate::service::get_client_connection_map;
use crate::service::handler::{call_handler_list, IOTaskSender};
use crate::service::server::InnerValue;

pub(super) async fn handler_func(
    sender: MsgMpmcSender,
    mut receiver: MsgMpscReceiver,
    io_task_sender: IOTaskSender,
    mut timeout_receiver: MsgMpscReceiver,
    server_info: &ServerInfo,
    handler_list: &HandlerList<InnerValue>,
    inner_state: &mut AHashMap<String, InnerValue>,
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
    let io_sender = sender.clone();
    let scheduler_id = server_info.id;
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
    let sender = MsgSender::Client(sender);
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
                    inner_state,
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
