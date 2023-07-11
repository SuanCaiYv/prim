use std::sync::Arc;

use async_trait::async_trait;
use lib_net_tokio::net::{Handler, ReqwestHandler};
use tracing::error;

use lib::{
    entity::{Msg, ReqwestMsg, ServerInfo, Type},
    error::HandlerError,
    net::InnerStates,
    Result,
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        let new_peer = msg.payload()[0] == 1;
        let server_info = ServerInfo::from(&(msg.payload())[1..]);
        crate::cluster::node_online(
            server_info.cluster_address.unwrap(),
            server_info.id,
            new_peer,
        )
        .await?;
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl ReqwestHandler for NodeUnregister {
    async fn run(&self, msg: &mut ReqwestMsg, _states: &mut InnerStates) -> Result<ReqwestMsg> {
        let node_info = ServerInfo::from(msg.payload());
        crate::cluster::node_offline(node_info.id).await?;
        Ok(ReqwestMsg::default())
    }
}

pub(crate) struct MessageForward {
    pub(crate) handler_list: Vec<Box<dyn Handler>>,
}

#[async_trait]
impl ReqwestHandler for MessageForward {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let msg = Msg::from(req.payload());
        let mut msg = Arc::new(msg);
        for handler in self.handler_list.iter() {
            match handler.run(&mut msg, states).await {
                Ok(ok_msg) => match ok_msg.typ() {
                    Type::Noop => {
                        continue;
                    }
                    _ => {
                        return Ok(ReqwestMsg::default());
                    }
                },
                Err(e) => {
                    match e.downcast::<HandlerError>() {
                        Ok(handler_err) => match handler_err {
                            HandlerError::NotMine => {
                                continue;
                            }
                            HandlerError::Auth { .. } => {
                                let res_msg = ReqwestMsg::with_resource_id_payload(
                                    req.resource_id(),
                                    b"auth failed",
                                );
                                return Ok(res_msg);
                            }
                            HandlerError::Parse(cause) => {
                                let res_msg = ReqwestMsg::with_resource_id_payload(
                                    req.resource_id(),
                                    cause.as_bytes(),
                                );
                                return Ok(res_msg);
                            }
                            HandlerError::IO(e) => {
                                error!("io error: {}", e);
                                let res_msg = ReqwestMsg::with_resource_id_payload(
                                    req.resource_id(),
                                    b"io error",
                                );
                                return Ok(res_msg);
                            }
                            HandlerError::Other(e) => {
                                error!("other error: {}", e);
                                let res_msg = ReqwestMsg::with_resource_id_payload(
                                    req.resource_id(),
                                    b"other error",
                                );
                                return Ok(res_msg);
                            }
                        },
                        Err(e) => {
                            error!("unhandled error: {}", e);
                            let res_msg = ReqwestMsg::with_resource_id_payload(
                                req.resource_id(),
                                b"unhandled error",
                            );
                            return Ok(res_msg);
                        }
                    };
                }
            }
        }
        Ok(ReqwestMsg::default())
    }
}
