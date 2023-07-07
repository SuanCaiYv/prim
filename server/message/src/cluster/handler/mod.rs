pub(super) mod logic;
pub(super) mod pure_text;
pub(super) mod forward;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use lib::{
    entity::{Msg, Type},
    net::{
        InnerStates, InnerStatesValue, GenericParameterMap,
    },
    Result,
};
use lib_net_tokio::net::{MsgMpscReceiver, HandlerList};
use tracing::{debug, error};

use crate::{
    cache::get_redis_ops,
    service::{
        get_client_connection_map,
        handler::{call_handler_list, IOTaskSender},
    },
    util::my_id,
};

use super::{get_cluster_connection_map, MsgSender};

pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    io_task_sender: &IOTaskSender,
    handler_list: &HandlerList,
    inner_states: &mut InnerStates,
) -> Result<()> {
    let cluster_map = get_cluster_connection_map().0;
    let mut generic_map = GenericParameterMap(AHashMap::new());
    generic_map.put_parameter(get_redis_ops().await);
    generic_map.put_parameter(get_client_connection_map());
    generic_map.put_parameter(io_task_sender.clone());
    generic_map.put_parameter(get_cluster_connection_map());
    generic_map.put_parameter(sender.clone());
    inner_states.insert("generic_map".to_string(), InnerStatesValue::GenericParameterMap(generic_map));
    let cluster_id;
    match receiver.recv().await {
        Some(mut auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let auth_handler = &handler_list[0];
            match auth_handler
                .run(&mut auth_msg, inner_states)
                .await
            {
                Ok(res_msg) => {
                    sender.send(Arc::new(res_msg)).await?;
                    cluster_id = auth_msg.sender() as u32;
                }
                Err(_) => {
                    let err_msg = Msg::err_msg(my_id() as u64, auth_msg.sender(), 0, "auth failed");
                    sender.send(Arc::new(err_msg)).await?;
                    return Err(anyhow!("auth failed"));
                }
            }
        }
        None => {
            error!("cannot receive auth message");
            return Err(anyhow!("cannot receive auth message"));
        }
    };
    loop {
        let msg = receiver.recv().await;
        match msg {
            Some(mut msg) => {
                call_handler_list(
                    &sender,
                    &mut msg,
                    handler_list,
                    inner_states,
                )
                .await?;
            }
            None => {
                // warn!("io receiver closed");
                debug!("connection closed");
                break;
            }
        }
    }
    cluster_map.remove(&cluster_id);
    Ok(())
}