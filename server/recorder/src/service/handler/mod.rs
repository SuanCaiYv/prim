mod business;
mod internal;
pub(crate) mod logic;
mod message;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use lib::entity::{Msg, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, Handler, HandlerList, WrapInnerSender};
use lib::net::InnerSender;
use lib::util::who_we_are;
use lib::{
    net::{server::HandlerParameters, OuterReceiver, OuterSender},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::config::CONFIG;
use crate::util::my_id;

use self::business::{Group, Relationship};
use self::logic::{Auth, Echo};
use self::message::Text;

use super::get_client_connection_map;

pub(super) async fn handler_func(mut io_channel: (OuterSender, OuterReceiver)) -> Result<()> {
    let client_map = get_client_connection_map().0;
    // todo
    let io_task_channel: (InnerSender, OuterReceiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_receiver_side_channel_size * 123);
    tokio::spawn(async move {
        if let Err(e) = io_task(io_task_channel.1).await {
            error!("io task error: {}", e);
        }
    });
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
        .put_parameter(WrapInnerSender(io_task_channel.0));
    let user_id;
    match io_channel.1.recv().await {
        Some(auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let auth_handler: Box<dyn Handler> = Box::new(Auth {});
            match auth_handler
                .run(auth_msg.clone(), &mut handler_parameters)
                .await
            {
                Ok(res_msg) => {
                    io_channel.0.send(Arc::new(res_msg)).await?;
                    client_map.insert(auth_msg.sender(), io_channel.0.clone());
                    user_id = auth_msg.sender();
                }
                Err(_) => {
                    let err_msg =
                        Msg::err_msg_str(my_id() as u64, auth_msg.sender(), 0, "auth failed");
                    io_channel.0.send(Arc::new(err_msg)).await?;
                    return Err(anyhow!("auth failed"));
                }
            }
        }
        None => {
            error!("cannot receive auth message");
            return Err(anyhow!("cannot receive auth message"));
        }
    }
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Echo {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Text {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Relationship {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Group {}));
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
    client_map.remove(&user_id);
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

/// only messages that need to be persisted into disk or cached into cache will be sent to this task.
/// those messages type maybe: all message part included/all business part included
pub(self) async fn io_task(mut io_task_receiver: OuterReceiver) -> Result<()> {
    loop {
        match io_task_receiver.recv().await {
            Some(msg) => {
                let msg_key = who_we_are(msg.sender(), msg.receiver());
            },
            None => {
                error!("io task receiver closed");
                return Err(anyhow!("io task receiver closed"));
            }
        }
    }
}
