mod msg;

use std::sync::Arc;

use ahash::AHashMap;
use lib::entity::{Msg, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, HandlerList};
use lib::{
    net::{server::HandlerParameters, OuterReceiver, OuterSender},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::util::my_id;

use self::msg::Message;

pub(super) async fn  handler_func(mut io_channel: (OuterSender, OuterReceiver)) -> Result<()> {
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    handler_parameters
        .generic_parameters
        .put_parameter(get_redis_ops().await);
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Message {}));
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
                        let mut ack_msg = msg.generate_ack();
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
                            let res_msg = Msg::err_msg(
                                my_id() as u64,
                                msg.sender(),
                                my_id(),
                                "auth failed",
                            );
                            io_channel.0.send(Arc::new(res_msg)).await?;
                        }
                        HandlerError::Parse(cause) => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), &cause);
                            io_channel.0.send(Arc::new(res_msg)).await?;
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
                        io_channel.0.send(Arc::new(res_msg)).await?;
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}
