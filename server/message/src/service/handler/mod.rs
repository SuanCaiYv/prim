use std::{any::Any, sync::Arc};

use ahash::AHashMap;
use anyhow::anyhow;
use dashmap::DashMap;
use lazy_static::lazy_static;
use lib::{
    entity::{Msg, Type, GROUP_ID_THRESHOLD},
    error::HandlerError,
    net::{
        server::{GenericParameter, GenericParameterMap, HandlerList},
        InnerStates, InnerStatesValue, MsgMpscReceiver, MsgSender,
    },
    util::{timestamp, who_we_are},
    Result,
};
use tracing::{debug, error};

use crate::{
    cache::{get_redis_ops, LAST_ONLINE_TIME, MSG_CACHE, USER_INBOX},
    cluster::get_cluster_connection_map,
    config::CONFIG,
    get_io_task_sender, rpc,
    util::my_id,
};

use super::get_client_connection_map;

pub(crate) mod business;
pub(crate) mod control_text;
pub(crate) mod logic;
pub(crate) mod pure_text;

pub(self) type GroupTaskSender = tokio::sync::mpsc::Sender<(Arc<Msg>, bool)>;
pub(self) type GroupTaskReceiver = tokio::sync::mpsc::Receiver<(Arc<Msg>, bool)>;

#[derive(Clone)]
pub(crate) struct IOTaskSender(pub(crate) tokio::sync::mpsc::Sender<IOTaskMsg>);

pub(crate) struct IOTaskReceiver(pub(crate) tokio::sync::mpsc::Receiver<IOTaskMsg>);

#[derive(Debug, Clone)]
pub(crate) enum IOTaskMsg {
    Direct(Arc<Msg>),
    Broadcast(Arc<Msg>, u64),
}

impl GenericParameter for IOTaskSender {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl GenericParameter for IOTaskReceiver {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl IOTaskSender {
    pub(crate) async fn send(&self, msg: IOTaskMsg) -> Result<()> {
        if let Err(e) = self.0.send(msg).await {
            return Err(anyhow!(e.to_string()));
        }
        Ok(())
    }
}

impl IOTaskReceiver {
    pub(crate) async fn recv(&mut self) -> Option<IOTaskMsg> {
        self.0.recv().await
    }
}

lazy_static! {
    static ref GROUP_SENDER_MAP: Arc<DashMap<u64, GroupTaskSender>> = Arc::new(DashMap::new());
    /// only represents the current node's group id and user id list
    static ref GROUP_USER_LIST: Arc<DashMap<u64, Vec<u64>>> = Arc::new(DashMap::new());
}

/// ```
///  -------------------------
/// |                         |
/// | quic stream established |
/// |                         |
///  -------------------------
///            | |
///           \   /
///            \ /
///  -------------------------
/// |                         |
/// |  auth message received  |
/// |                         |
///  -------------------------
///            | |
///           \   /
///            \ /
///  -------------------------
/// |                         |
/// |  next message received  |
/// |  more auth message,skip |
///  -------------------------
///            | |
///           \   /
///            \ /
///  -------------------------
/// |                         |
/// |      preprocessing      |
/// |                         |
///  -------------------------
///            | |
///           \   /
///            \ /
///  -------------------------
/// |                         |
/// |      handler run        |
/// |                         |
///  -------------------------
///            | |
///           \   /
///            \ /
///  -------------------------
/// |                         |
/// |       write back        |
/// |                         |
///  -------------------------
/// ```
/// this function is used to deal some prepare work before actually start the message stream call.
pub(super) async fn handler_func(
    sender: MsgSender,
    mut receiver: MsgMpscReceiver,
    io_task_sender: IOTaskSender,
    handler_list: &HandlerList,
    states: &mut InnerStates,
) -> Result<()> {
    let mut generic_map = GenericParameterMap(AHashMap::new());
    let client_map = get_client_connection_map().0;
    let mut redis_ops = get_redis_ops().await;
    generic_map.put_parameter(get_redis_ops().await);
    generic_map.put_parameter(get_client_connection_map());
    generic_map.put_parameter(io_task_sender);
    generic_map.put_parameter(get_cluster_connection_map());
    generic_map.put_parameter(sender.clone());
    states.insert(
        "generic_map".to_owned(),
        InnerStatesValue::GenericParameterMap(generic_map),
    );
    let user_id;
    match receiver.recv().await {
        Some(mut auth_msg) => {
            if auth_msg.typ() != Type::Auth {
                return Err(anyhow!("auth failed"));
            }
            // todo magic number should not be used.
            let auth_handler = &handler_list[0];
            match auth_handler.run(&mut auth_msg, states).await {
                Ok(res_msg) => {
                    sender.send(Arc::new(res_msg)).await?;
                    user_id = auth_msg.sender();
                }
                Err(e) => {
                    error!("auth handler error: {}", e);
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
                call_handler_list(&sender, &mut msg, handler_list, states).await?;
            }
            None => {
                // warn!("io receiver closed");
                debug!("connection closed");
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

/// this function is used to deal with logic/business message received from client.
#[inline(always)]
pub(crate) async fn call_handler_list(
    sender: &MsgSender,
    msg: &mut Arc<Msg>,
    handler_list: &HandlerList,
    states: &mut InnerStates,
) -> Result<()> {
    for handler in handler_list.iter() {
        match handler.run(msg, states).await {
            Ok(ok_msg) => {
                match ok_msg.typ() {
                    Type::Noop => {
                        continue;
                    }
                    Type::Ack => {
                        sender.send(Arc::new(ok_msg)).await?;
                    }
                    _ => {
                        let seq_num = ok_msg.seq_num();
                        sender.send(Arc::new(ok_msg)).await?;
                        let client_timestamp =
                            states.get("client_timestamp").unwrap().as_num().unwrap();
                        let mut ack_msg = msg.generate_ack(my_id(), client_timestamp);
                        ack_msg.set_sender(my_id() as u64);
                        ack_msg.set_receiver(msg.sender());
                        ack_msg.set_seq_num(seq_num);
                        sender.send(Arc::new(ack_msg)).await?;
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
                            sender.send(Arc::new(res_msg)).await?;
                        }
                        HandlerError::Parse(cause) => {
                            let res_msg =
                                Msg::err_msg(my_id() as u64, msg.sender(), my_id(), &cause);
                            sender.send(Arc::new(res_msg)).await?;
                        }
                    },
                    Err(e) => {
                        error!("unhandled error: {}", e);
                        let res_msg =
                            Msg::err_msg(my_id() as u64, msg.sender(), my_id(), "unhandled error");
                        sender.send(Arc::new(res_msg)).await?;
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

/// only messages that need to be persisted into disk or cached into cache will be sent to this task.
/// those messages types maybe: all message part / all business part
pub(super) async fn io_task(mut io_task_receiver: IOTaskReceiver) -> Result<()> {
    let mut redis_ops = get_redis_ops().await;
    // let recorder_sender = recorder_sender();
    loop {
        match io_task_receiver.recv().await {
            Some(task_msg) => {
                let users_identify;
                let msg: Arc<Msg>;
                let receiver: u64;
                match task_msg {
                    IOTaskMsg::Direct(direct_msg) => {
                        if is_group_msg(direct_msg.receiver()) {
                            users_identify =
                                who_we_are(direct_msg.receiver(), direct_msg.receiver())
                        } else {
                            users_identify = who_we_are(direct_msg.sender(), direct_msg.receiver());
                        }
                        receiver = direct_msg.receiver();
                        msg = direct_msg;
                    }
                    IOTaskMsg::Broadcast(broadcast_msg, real_receiver) => {
                        users_identify = who_we_are(broadcast_msg.sender(), real_receiver);
                        receiver = real_receiver;
                        msg = broadcast_msg;
                    }
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
                        &format!("{}{}", USER_INBOX, receiver),
                        &msg.sender(),
                        msg.timestamp() as f64,
                    )
                    .await?;
                // recorder_sender.send(msg).await?;
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
    let group_id = msg.receiver();
    match GROUP_SENDER_MAP.get(&group_id) {
        Some(io_sender) => {
            io_sender.send((msg.clone(), forward)).await?;
        }
        None => {
            // todo reset size
            let (io_sender, io_receiver) = tokio::sync::mpsc::channel(1024);
            io_sender.send((msg.clone(), forward)).await?;
            GROUP_SENDER_MAP.insert(group_id, io_sender);
            tokio::spawn(async move {
                if let Err(e) = group_task(group_id, io_receiver).await {
                    error!("group_task error: {}", e);
                    GROUP_SENDER_MAP.remove(&group_id);
                }
            });
        }
    }
    Ok(())
}

async fn load_group_user_list(group_id: u64) -> Result<()> {
    let mut rpc_client = rpc::get_rpc_client().await;
    let list = rpc_client.call_curr_node_group_id_user_list(group_id).await;
    if let Err(e) = list {
        error!("load group user list error: {}", e);
        return Err(anyhow!("load group user list error: {}", e));
    }
    let list = list.unwrap();
    GROUP_USER_LIST.insert(group_id, list);
    Ok(())
}

pub(self) async fn group_task(group_id: u64, mut io_receiver: GroupTaskReceiver) -> Result<()> {
    debug!("group task {} start", group_id);
    if let Err(e) = load_group_user_list(group_id).await {
        error!("load group user list error: {}", e);
    }
    let client_map = get_client_connection_map().0;
    let cluster_map = get_cluster_connection_map().0;
    let io_task_sender = get_io_task_sender();
    loop {
        match io_receiver.recv().await {
            Some((msg, forward)) => {
                if forward {
                    for entry in cluster_map.iter() {
                        match entry.value() {
                            MsgSender::Client(sender) => match sender.send(msg.clone()).await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("send to {} failed: {}", entry.key(), e);
                                }
                            },
                            MsgSender::Server(sender) => match sender.send(msg.clone()).await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("send to {} failed: {}", entry.key(), e);
                                }
                            },
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
                            if let Err(_) = io_task_sender
                                .send(IOTaskMsg::Broadcast(msg.clone(), *user_id))
                                .await
                            {
                                error!("send to io task failed");
                            }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[tokio::test]
    async fn test() {
        #[derive(Debug)]
        struct S {
            v1: i32,
            v2: i32,
        }
        let s = S { v1: 1, v2: 2 };
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(async move {
            loop {
                let v = rx.recv().await;
                println!("v: {:?}", v);
            }
        });
        let mut s = Arc::new(s);
        for i in 0..5 {
            let ss = Arc::get_mut(&mut s).unwrap();
            ss.v1 = i;
            ss.v2 = i * i;
            tx.send(s.clone()).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
