pub(crate) mod business;
pub(crate) mod control_text;
pub(crate) mod logic;
pub(crate) mod pure_text;

use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::cache::redis_ops::RedisOps;
use lib::entity::{Msg, Type, GROUP_ID_THRESHOLD};
use lib::error::HandlerError;
use lib::net::server::{GenericParameterMap, Handler, HandlerList, WrapInnerSender};
use lib::net::InnerSender;
use lib::util::{timestamp, who_we_are};
use lib::{
    net::{server::HandlerParameters, OuterReceiver, OuterSender},
    Result,
};
use tracing::{debug, error, info, warn};

use crate::cache::{get_redis_ops, LAST_ONLINE_TIME, MSG_CACHE, SEQ_NUM, USER_INBOX};
use crate::cluster::get_cluster_connection_map;
use crate::config::CONFIG;
use crate::recorder::recorder_sender;
use crate::rpc;
use crate::util::my_id;

use self::business::{AddFriend, JoinGroup, LeaveGroup, RemoveFriend, SystemMessage};
use self::logic::{Auth, Echo};
use self::pure_text::PureText;

use super::get_client_connection_map;

pub(self) type GroupTaskSender = tokio::sync::mpsc::Sender<(Arc<Msg>, bool)>;
pub(self) type GroupTaskReceiver = tokio::sync::mpsc::Receiver<(Arc<Msg>, bool)>;

lazy_static! {
    static ref GROUP_SENDER_MAP: Arc<DashMap<u64, GroupTaskSender>> = Arc::new(DashMap::new());
    /// only represents the current node's group id and user id list
    static ref GROUP_USER_LIST: Arc<DashMap<u64, Vec<u64>>> = Arc::new(DashMap::new());
}

pub(super) async fn handler_func(
    mut io_channel: (OuterSender, OuterReceiver),
    io_task_sender: InnerSender,
) -> Result<()> {
    let client_map = get_client_connection_map().0;
    let mut redis_ops = get_redis_ops().await;
    let mut handler_parameters = HandlerParameters {
        generic_parameters: GenericParameterMap(AHashMap::new()),
    };
    handler_parameters
        .generic_parameters
        .put_parameter(get_redis_ops().await);
    handler_parameters
        .generic_parameters
        .put_parameter(get_client_connection_map());
    handler_parameters
        .generic_parameters
        .put_parameter(WrapInnerSender(io_task_sender));
    handler_parameters
        .generic_parameters
        .put_parameter(get_cluster_connection_map());
    let user_id;
    match io_channel.1.recv().await {
        Some(auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            let auth_handler: Box<dyn Handler> = Box::new(Auth {});
            match auth_handler
                .run(auth_msg.clone(), &mut handler_parameters)
                .await
            {
                Ok(res_msg) => {
                    io_channel.0.send(Arc::new(res_msg)).await?;
                    client_map.insert(auth_msg.sender(), io_channel.0.clone());
                    user_id = auth_msg.sender();
                }
                Err(_) => {
                    let err_msg = Msg::err_msg(my_id() as u64, auth_msg.sender(), 0, "auth failed");
                    io_channel.0.send(Arc::new(err_msg)).await?;
                    return Err(anyhow!("auth failed"));
                }
            }
        }
        None => {
            error!("cannot receive auth message");
            return Err(anyhow!("cannot receive auth message"));
        }
    }
    let mut handler_list = HandlerList::new(Vec::new());
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(Echo {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(PureText {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(JoinGroup {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(LeaveGroup {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(AddFriend {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(RemoveFriend {}));
    Arc::get_mut(&mut handler_list)
        .unwrap()
        .push(Box::new(SystemMessage {}));
    loop {
        let msg = io_channel.1.recv().await;
        match msg {
            Some(msg) => {
                call_handler_list(&io_channel, msg, &handler_list, &mut handler_parameters).await?;
            }
            None => {
                info!("io receiver closed");
                warn!("connection closed");
                break;
            }
        }
    }
    client_map.remove(&user_id);
    // we choose to use [now - last idle timeout] to be the last online time.
    redis_ops
        .set(
            &format!("{}{}", LAST_ONLINE_TIME, user_id),
            &(timestamp() - CONFIG.transport.connection_idle_timeout),
        )
        .await?;
    Ok(())
}

pub(crate) async fn call_handler_list(
    io_channel: &(OuterSender, OuterReceiver),
    msg: Arc<Msg>,
    handler_list: &HandlerList,
    handler_parameters: &mut HandlerParameters,
) -> Result<()> {
    let msg = set_seq_num(
        msg,
        handler_parameters
            .generic_parameters
            .get_parameter_mut::<RedisOps>()?,
    )
    .await?;
    for handler in handler_list.iter() {
        match handler.run(msg.clone(), handler_parameters).await {
            Ok(ok_msg) => {
                match ok_msg.typ() {
                    Type::Noop => {}
                    Type::Ack => {
                        io_channel.0.send(Arc::new(ok_msg)).await?;
                    }
                    _ => {
                        io_channel.0.send(Arc::new(ok_msg)).await?;
                        let mut ack_msg = msg.generate_ack(my_id());
                        ack_msg.set_sender(my_id() as u64);
                        ack_msg.set_receiver(msg.sender());
                        // todo()!
                        ack_msg.set_seq_num(0);
                        io_channel.0.send(Arc::new(ack_msg)).await?;
                    }
                }
                break;
            }
            Err(e) => {
                match e.downcast::<HandlerError>() {
                    Ok(handler_err) => match handler_err {
                        HandlerError::NotMine => {
                            continue;
                        }
                        HandlerError::Auth { .. } => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), "auth failed");
                            io_channel.0.send(Arc::new(res_msg)).await?;
                        }
                        HandlerError::Parse(cause) => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), &cause);
                            io_channel.0.send(Arc::new(res_msg)).await?;
                        }
                    },
                    Err(e) => {
                        error!("unhandled error: {}", e);
                        let res_msg =
                            Msg::err_msg(my_id() as u64, msg.sender(), my_id(), "unhandled error");
                        io_channel.0.send(Arc::new(res_msg)).await?;
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}

#[inline]
pub(crate) fn is_group_msg(user_id: u64) -> bool {
    user_id >= GROUP_ID_THRESHOLD
}

pub(crate) async fn set_seq_num(mut msg: Arc<Msg>, redis_ops: &mut RedisOps) -> Result<Arc<Msg>> {
    let type_value = msg.typ().value();
    if type_value >= 32 && type_value < 64
        || type_value >= 64 && type_value < 96
        || type_value >= 128 && type_value < 160
    {
        let seq_num;
        if is_group_msg(msg.receiver()) {
            seq_num = redis_ops
                .atomic_increment(&format!(
                    "{}{}",
                    SEQ_NUM,
                    who_we_are(msg.receiver(), msg.receiver())
                ))
                .await?;
        } else {
            seq_num = redis_ops
                .atomic_increment(&format!(
                    "{}{}",
                    SEQ_NUM,
                    who_we_are(msg.sender(), msg.receiver())
                ))
                .await?;
        }
        match Arc::get_mut(&mut msg) {
            Some(msg) => {
                msg.set_seq_num(seq_num);
            }
            None => {
                return Err(anyhow!("cannot get mutable reference of msg"));
            }
        };
    }
    Ok(msg)
}

/// only messages that need to be persisted into disk or cached into cache will be sent to this task.
/// those messages type maybe: all message part included/all business part included
pub(super) async fn io_task(mut io_task_receiver: OuterReceiver) -> Result<()> {
    let mut redis_ops = get_redis_ops().await;
    let recorder_sender = recorder_sender();
    loop {
        match io_task_receiver.recv().await {
            Some(msg) => {
                let users_identify;
                if is_group_msg(msg.receiver()) {
                    users_identify = who_we_are(msg.receiver(), msg.receiver())
                } else {
                    users_identify = who_we_are(msg.sender(), msg.receiver());
                }
                // todo delete old data
                redis_ops
                    .push_sort_queue(
                        &format!("{}{}", MSG_CACHE, users_identify),
                        &msg.as_slice(),
                        msg.seq_num() as f64,
                    )
                    .await?;
                redis_ops
                    .push_sort_queue(
                        &format!("{}{}", USER_INBOX, msg.receiver()),
                        &msg.sender(),
                        msg.timestamp() as f64,
                    )
                    .await?;
                recorder_sender.send(msg).await?;
            }
            None => {
                error!("io task receiver closed");
                return Err(anyhow!("io task receiver closed"));
            }
        }
    }
}

/// forward: true if the message need to broadcast to all nodes(imply it comes from client), false if the message comes from other nodes.
pub(crate) async fn push_group_msg(msg: Arc<Msg>, forward: bool) -> Result<()> {
    let receiver = msg.receiver();
    match GROUP_SENDER_MAP.get(&receiver) {
        Some(io_sender) => {
            io_sender.send((msg.clone(), forward)).await?;
        }
        None => {
            let (io_sender, io_receiver) =
                tokio::sync::mpsc::channel(CONFIG.performance.max_receiver_side_channel_size);
            io_sender.send((msg.clone(), forward)).await?;
            GROUP_SENDER_MAP.insert(receiver, io_sender);
            tokio::spawn(async move {
                if let Err(e) = group_task(receiver, io_receiver).await {
                    error!("group_task error: {}", e);
                    GROUP_SENDER_MAP.remove(&receiver);
                }
            });
        }
    }
    Ok(())
}

async fn load_group_user_list(group_id: u64) -> Result<()> {
    let mut rpc_client = rpc::get_rpc_client().await;
    let list = rpc_client
        .call_curr_node_group_id_user_list(group_id)
        .await?;
    GROUP_USER_LIST.insert(group_id, list);
    Ok(())
}

pub(self) async fn group_task(group_id: u64, mut io_receiver: GroupTaskReceiver) -> Result<()> {
    debug!("group task {} start", group_id);
    load_group_user_list(group_id).await?;
    let client_map = get_client_connection_map().0;
    let cluster_map = get_cluster_connection_map().0;
    loop {
        match io_receiver.recv().await {
            Some((msg, forward)) => {
                if forward {
                    for entry in cluster_map.iter() {
                        match entry.value().send(msg.clone()).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("send to {} failed: {}", entry.key(), e);
                            }
                        }
                    }
                }
                // when send to clients, the message need sender set to group id first.
                // the truly sender will be set in extension part by original client.
                let mut new_msg = (*msg).clone();
                new_msg.set_sender(msg.receiver());
                new_msg.set_receiver(0);
                let msg = Arc::new(new_msg);
                match GROUP_USER_LIST.get(&group_id) {
                    Some(user_list) => {
                        for user_id in user_list.iter() {
                            if let Some(io_sender) = client_map.get(user_id) {
                                match io_sender.send(msg.clone()).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        debug!("send to {} failed: {}", user_id, e);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        error!("group {} not found", group_id);
                        return Err(anyhow!("group {} not found", group_id));
                    }
                }
            }
            None => {
                debug!("group task exit");
                break;
            }
        }
    }
    Ok(())
}
