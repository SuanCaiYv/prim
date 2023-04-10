use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use lib::entity::Type;
use lib::net::server::InnerStates;
use lib::{
    entity::Msg,
    error::HandlerError,
    net::server::{Handler, HandlerParameters},
    Result,
};
use tracing::{debug, error};

use crate::{cluster::ClusterConnectionMap, service::ClientConnectionMap, util::my_id};

use super::IOTaskSender;

#[inline]
pub(self) async fn forward_only_user(
    msg: Arc<Msg>,
    parameters: &mut HandlerParameters,
    inner_states: &mut InnerStates,
) -> Result<Msg> {
    let client_map = parameters
        .generic_parameters
        .get_parameter::<ClientConnectionMap>()?;
    let cluster_map = parameters
        .generic_parameters
        .get_parameter::<ClusterConnectionMap>()?;
    let io_task_sender = parameters
        .generic_parameters
        .get_parameter::<IOTaskSender>()?;
    let receiver = msg.receiver();
    let node_id = msg.node_id();
    if node_id == my_id() {
        match client_map.get(&receiver) {
            Some(client_sender) => {
                client_sender.send(msg.clone()).await?;
            }
            None => {
                debug!("receiver {} not found", receiver);
            }
        }
        if let Err(_) = io_task_sender
            .send(super::IOTaskMsg::Direct(msg.clone()))
            .await
        {
            error!("io task sender disconnected!");
            return Err(anyhow!("io task sender disconnected!"));
        }
    } else {
        match cluster_map.get(&node_id) {
            Some(sender) => {
                sender.send(msg.clone()).await?;
            }
            None => {
                // todo cluster offline error handler.
                error!("cluster[{}] offline!", node_id);
            }
        }
    }
    let client_timestamp = inner_states
        .get("client_timestamp")
        .unwrap()
        .as_num()
        .unwrap();
    Ok(msg.generate_ack(my_id(), client_timestamp))
}

pub(crate) struct JoinGroup;

#[async_trait]
impl Handler for JoinGroup {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::JoinGroup {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}

pub(crate) struct LeaveGroup;

#[async_trait]
impl Handler for LeaveGroup {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::LeaveGroup {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}

pub(crate) struct AddFriend;

#[async_trait]
impl Handler for AddFriend {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::AddFriend {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}

pub(crate) struct RemoveFriend;

#[async_trait]
impl Handler for RemoveFriend {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::RemoveFriend {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}

pub(crate) struct SystemMessage;

#[async_trait]
impl Handler for SystemMessage {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::SystemMessage {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}

pub(crate) struct RemoteInvoke;

#[async_trait]
impl Handler for RemoteInvoke {
    async fn run(
        &self,
        msg: Arc<Msg>,
        _parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::RemoteInvoke {
            return Err(anyhow!(HandlerError::NotMine));
        }
        let client_timestamp = inner_states
            .get("client_timestamp")
            .unwrap()
            .as_num()
            .unwrap();
        Ok(msg.generate_ack(my_id(), client_timestamp))
    }
}

pub(crate) struct SetRelationship;

#[async_trait]
impl Handler for SetRelationship {
    async fn run(
        &self,
        msg: Arc<Msg>,
        parameters: &mut HandlerParameters,
        inner_states: &mut InnerStates,
    ) -> Result<Msg> {
        if msg.typ() != Type::SetRelationship {
            return Err(anyhow!(HandlerError::NotMine));
        }
        forward_only_user(msg, parameters, inner_states).await
    }
}
