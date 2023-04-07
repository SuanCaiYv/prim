mod message;

use std::sync::Arc;

use lib::entity::{Msg, ServerInfo, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, HandlerList};
use lib::{
    net::{server::HandlerParameters, MsgMpscReceiver},
    Result,
};

use ahash::AHashMap;
use tracing::error;

use crate::util::my_id;

use super::MsgSender;

pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    mut timeout: MsgMpscReceiver,
    server_info: &ServerInfo,
    handler_list: &HandlerList<()>,
    inner_state: &mut AHashMap<String, ()>,
) -> Result<()> {
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(message::NodeRegister {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(message::NodeUnregister {}));
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
                    &mut receiver,
                    msg,
                    &handler_list,
                    &mut handler_parameters,
                    inner_state,
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

async fn call_handler_list(
    sender: &MsgSender,
    _receiver: &mut MsgMpscReceiver,
    msg: Arc<Msg>,
    handler_list: &HandlerList<()>,
    handler_parameters: &mut HandlerParameters,
    inner_state: &mut AHashMap<String, ()>
) -> Result<()> {
    match sender {
        MsgSender::Client(sender) => {
            for handler in handler_list.iter() {
                match handler.run(msg.clone(), handler_parameters, inner_state).await {
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
                                    let res_msg = Msg::err_msg(
                                        my_id() as u64,
                                        msg.sender(),
                                        my_id(),
                                        "auth failed",
                                    );
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
                                let res_msg = Msg::err_msg(
                                    my_id() as u64,
                                    msg.sender(),
                                    my_id(),
                                    "unhandled error",
                                );
                                sender.send(Arc::new(res_msg)).await?;
                                break;
                            }
                        };
                    }
                }
            }
        }
        MsgSender::Server(sender) => {
            for handler in handler_list.iter() {
                match handler.run(msg.clone(), handler_parameters, inner_state).await {
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
                                    let res_msg = Msg::err_msg(
                                        my_id() as u64,
                                        msg.sender(),
                                        my_id(),
                                        "auth failed",
                                    );
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
                                let res_msg = Msg::err_msg(
                                    my_id() as u64,
                                    msg.sender(),
                                    my_id(),
                                    "unhandled error",
                                );
                                sender.send(Arc::new(res_msg)).await?;
                                break;
                            }
                        };
                    }
                }
            }
        }
    };
    Ok(())
}
