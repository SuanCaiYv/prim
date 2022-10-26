use anyhow::anyhow;
use async_trait::async_trait;

use common::entity::{Msg, Type};
use common::error::{CrashError, HandlerError};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::inner::{ConnectionId, StatusMap};
use common::net::server::{Handler, HandlerParameters};

pub(crate) struct Register;

#[async_trait]
impl Handler for Register {
    async fn run(&self, msg: Arc<Msg>, parameters: &mut HandlerParameters) -> common::Result<Msg> {
        if msg.typ() != Type::Register {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let generic_map = &mut parameters.generic_parameters;
        let connection_id;
        {
            let connection_id0 = generic_map.get_parameter_mut::<ConnectionId>();
            if connection_id0.is_err() {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "connection id not found".to_string()
                )));
            }
            connection_id = connection_id0.unwrap().0;
        }
        let status_map;
        {
            let status_map0 = generic_map.get_parameter_mut::<StatusMap>();
            if status_map0.is_err() {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "status map not found".to_string()
                )));
            }
            status_map = &status_map0.unwrap().0;
        }
        let node_info = status_map.get_mut(&connection_id);
        if node_info.is_none() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "node info not found".to_string()
            )));
        }
        let mut node_info = node_info.unwrap();
        node_info.connection_id = connection_id;
        let addr = String::from_utf8_lossy(msg.payload()).parse::<SocketAddr>();
        if addr.is_err() {
            return Err(anyhow!(HandlerError::Parse("addr not found".to_string())));
        }
        node_info.addr = addr.unwrap();
        Ok(msg.generate_ack(msg.timestamp()))
    }
}
