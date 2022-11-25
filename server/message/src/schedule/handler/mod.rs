mod internal;

use std::sync::Arc;

use ahash::AHashMap;
use lib::entity::{Msg, ServerInfo, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, HandlerList};
use lib::{
    net::{server::HandlerParameters, OuterReceiver, OuterSender},
    Result,
};
use tracing::error;

use crate::util::my_id;

pub(super) async fn handler_func(
    mut io_channel: (OuterSender, OuterReceiver),
    mut timeout_receiver: OuterReceiver,
    server_info: &ServerInfo,
) -> Result<()> {
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(internal::NodeRegister {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(internal::NodeUnregister {}));
    let io_sender = io_channel.0.clone();
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
                    error!("scheduler[{}] crashed.", scheduler_id);
                    break;
                }
            }
        }
    });
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    loop {
        let msg = io_channel.1.recv().await;
        match msg {
            Some(msg) => {
                call_handler_list(&io_channel, msg, &handler_list, &mut handler_parameters).await?;
            }
            None => {
                error!("io receiver closed");
                break;
            }
        }
    }
    Ok(())
}

async fn call_handler_list(
    io_channel: &(OuterSender, OuterReceiver),
    msg: Arc<Msg>,
    handler_list: &HandlerList,
    handler_parameters: &mut HandlerParameters,
) -> Result<()> {
    for handler in handler_list.iter() {
        match handler.run(msg.clone(), handler_parameters).await {
            Ok(ok_msg) => {
                match ok_msg.typ() {
                    Type::Noop => {
                        break;
                    }
                    Type::Ack => {
                        io_channel.0.send(Arc::new(ok_msg)).await?;
                    }
                    _ => {
                        io_channel.0.send(Arc::new(ok_msg)).await?;
                        let mut ack_msg = msg.generate_ack(msg.timestamp());
                        ack_msg.set_sender(my_id() as u64);
                        ack_msg.set_receiver(msg.sender());
                        // todo()!
                        ack_msg.set_seq_num(0);
                        io_channel.0.send(Arc::new(ack_msg)).await?;
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
                            let res_msg = Msg::err_msg_str(
                                my_id() as u64,
                                msg.sender(),
                                my_id(),
                                "auth failed",
                            );
                            io_channel.0.send(Arc::new(res_msg)).await?;
                        }
                        HandlerError::Parse(cause) => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), cause);
                            io_channel.0.send(Arc::new(res_msg)).await?;
                        }
                    },
                    Err(e) => {
                        error!("unhandled error: {}", e);
                        let res_msg = Msg::err_msg_str(
                            my_id() as u64,
                            msg.sender(),
                            my_id(),
                            "unhandled error",
                        );
                        io_channel.0.send(Arc::new(res_msg)).await?;
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}
