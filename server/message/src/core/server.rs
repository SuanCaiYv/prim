use crate::cache::get_redis_ops;
use crate::core::get_connection_map;
use ahash::AHashMap;
use async_trait::async_trait;
use common::entity::Msg;
use common::error::HandlerError;
use common::net::server::{
    GenericParameterMap, HandlerList, HandlerParameters, IOReceiver, IOSender, NewConnectionHandler,
};
use common::net::InnerSender;
use common::Result;
use jwt_simple::reexports::anyhow::anyhow;
use std::sync::Arc;
use tracing::error;

/// provide some external information.
pub(super) struct MessageConnectionHandler {
    pub(super) handler_list: HandlerList,
    pub(super) inner_sender: InnerSender,
}

impl MessageConnectionHandler {
    pub(super) fn new(
        handler_list: HandlerList,
        inner_sender: InnerSender,
    ) -> MessageConnectionHandler {
        MessageConnectionHandler {
            handler_list,
            inner_sender,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for MessageConnectionHandler {
    async fn handle(&mut self, mut io_channel: (IOSender, IOReceiver)) -> Result<()> {
        let mut handler_parameters = HandlerParameters {
            io_handler_sender: self.inner_sender.clone(),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        handler_parameters
            .generic_parameters
            .put_parameter(get_redis_ops().await);
        if let Some(auth_msg) = io_channel.1.recv().await {
            let auth_handler = &self.handler_list[0];
            match auth_handler
                .run(auth_msg.clone(), &mut handler_parameters)
                .await
            {
                Ok(res_msg) => {
                    io_channel.0.send(Arc::new(res_msg)).await?;
                }
                Err(e) => {
                    error!("auth failed: {}", e);
                    return Err(anyhow!("auth failed."));
                }
            };
            let connection_map = get_connection_map();
            connection_map
                .0
                .insert(auth_msg.sender(), io_channel.0.clone());
        } else {
            error!("auth msg not found.");
            return Err(anyhow!("auth msg not found."));
        }
        loop {
            if let Some(msg) = io_channel.1.recv().await {
                for handler in self.handler_list.iter() {
                    let res = handler.run(msg.clone(), &mut handler_parameters).await;
                    let mut res_msg = None;
                    match res {
                        Ok(success) => {
                            res_msg = Some(success);
                        }
                        Err(e) => {
                            let err = e.downcast::<HandlerError>();
                            match err {
                                Ok(err) => match err {
                                    HandlerError::NotMine => {
                                        continue;
                                    }
                                    HandlerError::Auth { .. } => {
                                        let msg = Msg::err_msg_str(
                                            0,
                                            msg.sender(),
                                            0,
                                            msg.sender_node(),
                                            "auth failed.",
                                        );
                                        res_msg = Some(msg);
                                        break;
                                    }
                                    HandlerError::Parse(cause) => {
                                        let msg = Msg::err_msg(
                                            0,
                                            msg.sender(),
                                            0,
                                            msg.sender_node(),
                                            cause,
                                        );
                                        res_msg = Some(msg);
                                        break;
                                    }
                                },
                                Err(_) => {
                                    error!("unhandled error: {}", err.as_ref().err().unwrap());
                                    continue;
                                }
                            }
                        }
                    }
                    if res_msg.is_none() {
                        res_msg = Some(Msg::err_msg_str(
                            0,
                            msg.sender(),
                            0,
                            msg.sender_node(),
                            "unknown msg type",
                        ));
                    }
                    if let Err(_) = io_channel.0.send(Arc::new(res_msg.unwrap())).await {
                        error!("send failed.");
                        return Err(anyhow!("send failed."));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
