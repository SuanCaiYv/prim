use crate::cache::get_redis_ops;
use crate::inner::{get_node_client_map, get_status_map};
use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use common::entity::{BalancerMode, Msg, Type};
use common::error::HandlerError;
use common::net::server::{
    GenericParameterMap, HandlerList, HandlerParameters, IOReceiver, IOSender, NewConnectionHandler,
};
use common::net::InnerSender;
use common::Result;
use serde::__private::from_utf8_lossy;
use std::sync::Arc;
use tracing::{error, info};

/// provide some external information.
pub(super) struct BalancerConnectionHandler {
    pub(super) handler_list: HandlerList,
    pub(super) inner_sender: InnerSender,
}

impl BalancerConnectionHandler {
    pub(super) fn new(
        handler_list: HandlerList,
        inner_sender: InnerSender,
    ) -> BalancerConnectionHandler {
        BalancerConnectionHandler {
            handler_list,
            inner_sender,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for BalancerConnectionHandler {
    async fn handle(&mut self, mut io_channel: (IOSender, IOReceiver)) -> Result<()> {
        let mut handler_parameters = HandlerParameters {
            io_handler_sender: self.inner_sender.clone(),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        handler_parameters
            .generic_parameters
            .put_parameter(get_status_map());
        handler_parameters
            .generic_parameters
            .put_parameter(get_redis_ops().await);
        let node_id;
        if let Some(auth_msg) = io_channel.1.recv().await {
            let auth_handler = &self.handler_list[0];
            match auth_handler
                .run(auth_msg.clone(), &mut handler_parameters)
                .await
            {
                Ok(res_msg) => {
                    node_id = auth_msg.sender() as u32;
                    io_channel.0.send(Arc::new(res_msg)).await?;
                }
                Err(e) => {
                    error!("auth failed: {}", e);
                    return Err(anyhow!("auth failed."));
                }
            };
            let msg_mode = BalancerMode::from(auth_msg.extension()[0]);
            match msg_mode {
                BalancerMode::Cluster => {
                    let addr_str = String::from_utf8_lossy
                }
                BalancerMode::Node => {
                    let node_client_map = get_node_client_map();
                    node_client_map
                        .0
                        .insert(auth_msg.sender() as u32, io_channel.0.clone());
                }
            }
        } else {
            error!("auth msg not found.");
            return Err(anyhow!("auth msg not found."));
        }
        loop {
            if let Some(msg) = io_channel.1.recv().await {
                let mut res_msg = None;
                for handler in self.handler_list.iter() {
                    let res = handler.run(msg.clone(), &mut handler_parameters).await;
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
                    break;
                }
            } else {
                info!("connection closed");
                break;
            }
        }
        let status_map = get_status_map().0;
        let node_client_map = get_node_client_map().0;
        if let Some(node_info) = status_map.get(&node_id) {
            let mut unregister_msg = Msg::raw_payload(&node_info.to_bytes());
            unregister_msg.set_type(Type::NodeUnregister);
            self.inner_sender.send(Arc::new(unregister_msg)).await?;
        }
        status_map.remove(&node_id);
        node_client_map.remove(&node_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {}
}
