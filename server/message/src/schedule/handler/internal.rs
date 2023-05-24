use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use tracing::error;

use crate::util::my_id;
use lib::{
    entity::{Msg, ReqwestMsg, ServerInfo, Type},
    error::HandlerError,
    net::{server::Handler, InnerStates, ReqwestHandler},
    Result,
};

pub(crate) struct NodeRegister {}

#[async_trait]
impl ReqwestHandler for NodeRegister {
    async fn run(&self, msg: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let new_peer = msg.payload()[0] == 1;
        let server_info = ServerInfo::from(&(msg.payload())[1..]);
        crate::cluster::node_online(server_info.cluster_address, server_info.id, new_peer).await?;
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}

pub(crate) struct NodeUnregister {}

#[async_trait]
impl ReqwestHandler for NodeUnregister {
    async fn run(&self, msg: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        if msg.typ() != Type::MessageNodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        crate::cluster::node_offline(msg.clone()).await?;
        Ok(msg.generate_ack(my_id(), msg.timestamp()))
    }
}

pub(crate) struct MessageForward {
    handler_list: Vec<Box<dyn Handler>>,
}

#[async_trait]
impl ReqwestHandler for MessageForward {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg> {
        let msg = Msg::from(req.payload());
        let mut msg = Arc::new(msg);
        for handler in self.handler_list.iter() {
            match handler.run(&mut msg, states).await {
                Ok(ok_msg) => match ok_msg.typ() {
                    Type::Noop => {}
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
    }
}
