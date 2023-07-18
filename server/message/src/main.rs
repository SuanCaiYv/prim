use lib::{joy, Result};
use structopt::StructOpt;
use tracing::{error, info};

use crate::service::handler::{IOTaskReceiver, IOTaskSender};
use crate::{
    config::{CONFIG, CONFIG_FILE_PATH},
    service::handler::IOTaskMsg,
    util::my_id,
};
use crate::service::{load_io_task, load_msglogger};

mod cache;
mod cluster;
mod config;
mod rpc;
mod schedule;
mod seqnum;
mod service;
mod util;

#[derive(StructOpt, Debug)]
#[structopt(name = "prim/message")]
pub(crate) struct Opt {
    #[structopt(
    long,
    long_help = r"provide you config.toml file by this option",
    default_value = "./message/config.toml"
    )]
    pub(crate) config: String,
    #[structopt(
    long = "my_id",
    long_help = r"manually set 'my_id' of server node",
    default_value = "0"
    )]
    pub(crate) my_id: u32,
}


/// the message node is the main service of prim, it is responsible for
/// receiving messages from the client, and other message nodes, and then
/// forwarding the messages to the corresponding message nodes.
///
/// there is a promise made by message node, that is, if client got a ack
/// for a message, then the message must be delivered to the corresponding
/// user.
///
/// to achieve this, node will persist the message to the disk, and then
/// forward the message to the corresponding node. or directly forward the
/// message to the corresponding user, after that, node will send a ack to
/// the client.
///
/// for the purpose of performance, forward behavior is asynchronous. so
/// how to ensure the promise?
///
/// if the destination node is down, all messages cached in sender channel
/// will be persisted to the disk. and the last acknowledged message will also be
/// persisted to the disk. when the destination node is up, scheduler will
/// notice current node, and then current node will send all blocked messages
/// judged by persistance information to the destination node.
///
/// but what happened if current node is down?
///
/// if current node is down, all messages cached in sender channel will lost.
/// so we need to handle messages that are persisted on disk but not handle
/// by downstream service, such as: database save, analyzing, etc. other messages
/// not on disk should be treated as send failed. and causing retry behavior for client.
/// so, which persisted messages should be re-send after node re-launch?
///
/// the answer is, other services will be needed to get last acknowledged message.
/// for peer message node, the re-launch node will ask all peer nodes that last message
/// sent by the re-launch node to get the last acknowledged message. for downstream services,
/// the re-launch node will ask the downstream service to get the last acknowledged message.
///
/// in our design, the downstream service is decoupled by message queue, such as kafka.
/// so the consumer of kafka for current node need to consume all messages to get the last
/// acknowledged message. this behavior requires scheduler to complete. and may block the
/// re-launch of node. only all cached message in kafka is consumed, the re-launch of node
/// can be completed, and all persisted messages but not handled by downstream service can
/// be re-send.
#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();
    unsafe { CONFIG_FILE_PATH = Box::leak(opt.config.into_boxed_str()) }
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init()
        .unwrap();
    util::load_my_id(opt.my_id).await?;
    // rpc::gen()?;
    println!("{}", joy::banner());
    info!(
        "prim message[{}] running on {}",
        my_id(),
        CONFIG.server.service_address
    );
    load_msglogger().await?;
    load_io_task();
    tokio::spawn(async move {
        if let Err(e) = cluster::start().await {
            error!("cluster error: {}", e);
        }
    });
    tokio::spawn(async move {
        if let Err(e) = schedule::start().await {
            error!("schedule error: {}", e);
        }
    });
    service::start().await?;
    Ok(())
}
