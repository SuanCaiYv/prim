use std::{net::SocketAddr, sync::Arc};

use crate::config::CONFIG;

use common::entity::Type;
use common::net::{InnerSender, OuterReceiver};
use common::util::timestamp;
use common::Result;
use dashmap::{DashMap, DashSet};
use delay_timer::prelude::{DelayTimerBuilder, TaskBuilder};
use lazy_static::lazy_static;
use tracing::error;

use self::server::Server;

mod cluster;
mod handler;
mod server;

pub(self) type ClusterConnectionSet = Arc<DashMap<SocketAddr, InnerSender>>;
pub(self) type ClientSenderMap = Arc<DashMap<u32, InnerSender>>;
pub(self) type AckMap = Arc<DashSet<String>>;

lazy_static! {
    pub(self) static ref CLUSTER_CONNECTION_SET: ClusterConnectionSet =
        ClusterConnectionSet::new(DashMap::new());
    pub(self) static ref CLIENT_SENDER_MAP: ClientSenderMap = ClientSenderMap::new(DashMap::new());
    pub(self) static ref ACK_MAP: AckMap = Arc::new(DashSet::new());
}

pub(self) async fn msg_handler(mut receiver: OuterReceiver) -> Result<()> {
    let timer = DelayTimerBuilder::default()
        .tokio_runtime_by_default()
        .build();
    let ack_map = ACK_MAP.clone();
    let client_map = CLIENT_SENDER_MAP.clone();
    loop {
        let msg = receiver.recv().await;
        if msg.is_none() {
            break;
        }
        let msg = msg.unwrap();
        if msg.typ() != Type::Replay {
            continue;
        }
        let mode_value = String::from_utf8_lossy(msg.extension()).parse::<u8>();
        if mode_value.is_err() {
            continue;
        }
        let mode_value = mode_value.unwrap();
        let mode = MsgMode::from(mode_value);
        match mode {
            MsgMode::Cluster => {}
            MsgMode::Origin => {
                let replay_id = String::from_utf8_lossy(msg.payload()).to_string();
                ack_map.insert(replay_id);
                let sender_key = msg.sender_node();
                let timeout_channel = client_map.get(&sender_key);
                if timeout_channel.is_none() {
                    continue;
                }
                let timeout_channel = timeout_channel.unwrap().clone();
                let task = TaskBuilder::default()
                    .set_task_id(timestamp())
                    .set_frequency_once_by_seconds(CONFIG.max_deal_time.as_secs())
                    .set_maximum_parallel_runnable_num(1)
                    .spawn_async_routine(move || {
                        let msg = msg.clone();
                        let sender = timeout_channel.clone();
                        async move {
                            let res = sender.send(msg).await;
                            if let Err(e) = res {
                                error!("error sending msg to client: {}", e);
                            }
                        }
                    });
                if task.is_err() {
                    error!("error creating task: {}", task.err().unwrap());
                } else {
                    let res = timer.add_task(task.unwrap());
                    if let Err(e) = res {
                        error!("error adding task to timer: {}", e);
                    }
                }
            }
            MsgMode::Target => {}
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgMode {
    Cluster,
    Origin,
    Target,
}

impl From<u8> for MsgMode {
    fn from(v: u8) -> Self {
        match v {
            1 => MsgMode::Cluster,
            2 => MsgMode::Origin,
            3 => MsgMode::Target,
            _ => panic!("invalid msg mode"),
        }
    }
}

impl MsgMode {
    #[allow(unused)]
    pub fn value(&self) -> u8 {
        match *self {
            MsgMode::Cluster => 1,
            MsgMode::Origin => 2,
            MsgMode::Target => 3,
        }
    }
}

pub(crate) async fn start() -> Result<()> {
    let global_channel = tokio::sync::mpsc::channel(256);
    tokio::spawn(async move {
        let res = Server::new().run(global_channel.0).await;
        if let Err(e) = res {
            error!("error running server: {}", e);
        }
    });
    tokio::spawn(async move {
        let res = cluster::Cluster::run().await;
        if let Err(e) = res {
            error!("error running cluster: {}", e);
        }
    });
    let res = msg_handler(global_channel.1).await;
    if let Err(e) = res {
        error!("error handling msg: {}", e);
    }
    Ok(())
}
