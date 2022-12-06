use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};

pub struct NodeRegister;

#[async_trait]
impl Handler for NodeRegister {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::RecorderNodeRegister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        Ok(msg.generate_ack())
    }
}

pub struct NodeUnregister;

#[async_trait]
impl Handler for NodeUnregister {
    async fn run(&self, msg: Arc<Msg>, _parameters: &mut HandlerParameters) -> Result<Msg> {
        if msg.typ() != Type::RecorderNodeUnregister {
            return Err(anyhow!(HandlerError::NotMine));
        }
        Ok(msg.generate_ack())
    }
}
