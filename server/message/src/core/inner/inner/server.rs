use std::sync::Arc;

use crate::cache::get_redis_ops;
use crate::core::get_cluster_connection_map;

use ahash::AHashMap;
use async_trait::async_trait;
use common::entity::{Msg, ServerInfo};
use common::error::HandlerError;
use common::net::server::{
    GenericParameterMap, HandlerList, HandlerParameters, IOReceiver, IOSender, NewConnectionHandler,
};
use common::net::InnerSender;
use common::Result;
use jwt_simple::reexports::anyhow::anyhow;
use tracing::error;

/// provide some external information.
pub(in crate::core) struct ClusterConnectionHandler {
    pub(in crate::core) handler_list: HandlerList,
    pub(in crate::core) inner_sender: InnerSender,
}

impl ClusterConnectionHandler {
    pub(in crate::core) fn new(
        handler_list: HandlerList,
        inner_sender: InnerSender,
    ) -> ClusterConnectionHandler {
        ClusterConnectionHandler {
            handler_list,
            inner_sender,
        }
    }
}

#[async_trait]
impl NewConnectionHandler for ClusterConnectionHandler {
    async fn handle(&mut self, mut io_channel: (IOSender, IOReceiver)) -> Result<()> {
        let mut handler_parameters = HandlerParameters {
            io_handler_sender: self.inner_sender.clone(),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        handler_parameters
            .generic_parameters
            .put_parameter(get_redis_ops().await);
        if let Some(first_msg) = io_channel.1.recv().await {
            let server_info = ServerInfo::from(first_msg.payload());
            let cluster_map = get_cluster_connection_map();
            cluster_map.insert(server_info.id, io_channel.0.clone());
        } else {
            error!("first msg not found.");
            return Err(anyhow!("first msg not found."));
        }
        loop {
            if let Some(msg) = io_channel.1.recv().await {
                let mut res_msg = None;
                for handler in self.handler_list.iter() {
                    let res = handler.run(msg.clone(), &mut handler_parameters).await;
                    res_msg = match res {
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
                        Some(_) => {
                            break;
                        }
                    }
                }
                match res_msg {
                    Some(res_msg) => {
                        if res_msg.is_no_op() {
                            continue;
                        }
                        if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
                            error!("send failed.");
                            return Err(anyhow!("send failed."));
                        }
                    }
                    None => {
                        let res_msg = Msg::err_msg_str(
                            0,
                            msg.sender(),
                            0,
                            msg.sender_node(),
                            "unknown msg type",
                        );
                        if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
                            error!("send failed.");
                            return Err(anyhow!("send failed."));
                        }
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
