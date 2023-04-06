mod msg;

use std::sync::Arc;

use ahash::AHashMap;
use lib::entity::{Msg, Type};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, HandlerList, WrapMsgMpscSender};
use lib::net::{MsgMpscSender, MsgSender};
use lib::{
    net::{server::HandlerParameters, MsgMpscReceiver},
    Result,
};
use tracing::error;

use crate::cache::get_redis_ops;
use crate::util::my_id;

use self::msg::Message;

use super::BUFFER_CHANNEL;

pub(super) async fn handler_func(
    sender: MsgMpscSender,
    mut receiver: MsgMpscReceiver,
) -> Result<()> {
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    handler_parameters
        .generic_parameters
        .put_parameter(get_redis_ops().await);
    handler_parameters
        .generic_parameters
        .put_parameter(WrapMsgMpscSender(BUFFER_CHANNEL.0.clone()));
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Message {}));
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

async fn call_handler_list(
    sender: &MsgSender,
    _receiver: &mut MsgMpscReceiver,
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
