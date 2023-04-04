mod internal;

use std::sync::Arc;

use ahash::AHashMap;
use lib::entity::ServerInfo;
use lib::net::server::{GenericParameterMap, HandlerList, WrapMsgMpscSender};
use lib::net::{MsgMpscSender, MsgSender};
use lib::{
    net::{server::HandlerParameters, MsgMpscReceiver, MsgMpmcSender},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::cluster::get_cluster_connection_map;
use crate::service::get_client_connection_map;
use crate::service::handler::{business, call_handler_list, control_text};

pub(super) async fn handler_func(
    sender: MsgMpmcSender,
    mut receiver: MsgMpscReceiver,
    io_task_sender: MsgMpscSender,
    mut timeout_receiver: MsgMpscReceiver,
    server_info: &ServerInfo,
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
        .put_parameter(WrapMsgMpscSender(io_task_sender));
    handler_parameters
        .generic_parameters
        .put_parameter(get_cluster_connection_map());
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(internal::NodeRegister {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(internal::NodeUnregister {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(control_text::ControlText {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(business::JoinGroup {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(business::LeaveGroup {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(business::AddFriend {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(business::RemoveFriend {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(business::SystemMessage {}));
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
                call_handler_list(&sender, &mut receiver, msg, &handler_list, &mut handler_parameters).await?;
            }
            None => {
                error!("io receiver closed");
                break;
            }
        }
    }
    Ok(())
}
