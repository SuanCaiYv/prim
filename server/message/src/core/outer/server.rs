use crate::cache::get_redis_ops;
use crate::core::{get_client_connection_map, get_group_recorded_user_id, get_group_user_list};
use crate::rpc::get_rpc_client;
use ahash::{AHashMap, AHashSet};
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
pub(in crate::core) struct ClientConnectionHandler {
    pub(in crate::core) handler_list: HandlerList,
    pub(in crate::core) inner_sender: InnerSender,
}

impl ClientConnectionHandler {
    pub(in crate::core) fn new(
        handler_list: HandlerList,
        inner_sender: InnerSender,
    ) -> ClientConnectionHandler {
        ClientConnectionHandler {
            handler_list,
            inner_sender,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for ClientConnectionHandler {
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
            let connection_map = get_client_connection_map();
            connection_map
                .0
                .insert(auth_msg.sender(), io_channel.0.clone());
            let recorder = get_group_recorded_user_id();
            let flag;
            {
                flag = recorder.0.contains(&auth_msg.sender());
            }
            if !flag {
                let group_user_list = get_group_user_list();
                let mut rpc_client = get_rpc_client().await;
                let list = rpc_client.call_user_group_list(auth_msg.sender()).await?;
                group_user_list
                    .0
                    .insert(auth_msg.sender(), AHashSet::from_iter(list));
                recorder.0.insert(auth_msg.sender());
            }
        } else {
            error!("auth msg not found.");
            return Err(anyhow!("auth msg not found."));
        }
        loop {
            if let Some(msg) = io_channel.1.recv().await {
                let mut can_deal = false;
                for handler in self.handler_list.iter() {
                    let res = handler.run(msg.clone(), &mut handler_parameters).await;
                    let res_msg = match res {
                        Ok(success) => Some(success),
                        Err(e) => {
                            let err = e.downcast::<HandlerError>();
                            match err {
                                Ok(err) => match err {
                                    HandlerError::NotMine => None,
                                    HandlerError::Auth { .. } => Some(Msg::err_msg_str(
                                        0,
                                        msg.sender(),
                                        0,
                                        msg.sender_node(),
                                        "auth failed.",
                                    )),
                                    HandlerError::Parse(cause) => Some(Msg::err_msg(
                                        0,
                                        msg.sender(),
                                        0,
                                        msg.sender_node(),
                                        cause,
                                    )),
                                },
                                Err(_) => {
                                    error!("unhandled error: {}", err.as_ref().err().unwrap());
                                    None
                                }
                            }
                        }
                    };
                    match res_msg {
                        None => {
                            continue;
                        }
                        Some(res_msg) => {
                            if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
                                error!("send failed.");
                                return Err(anyhow!("send failed."));
                            }
                            can_deal = true;
                            break;
                        }
                    }
                }
                if !can_deal {
                    let res_msg =
                        Msg::err_msg_str(0, msg.sender(), 0, msg.sender_node(), "unknown msg type");
                    if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
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
