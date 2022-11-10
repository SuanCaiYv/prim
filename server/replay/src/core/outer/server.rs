use std::sync::Arc;

use crate::config::CONFIG;
use crate::core::{get_ack_map, get_cluster_connection_set, AckMap, ClusterConnectionSet};
use anyhow::anyhow;
use common::entity::{Msg, ReplayMode, ServerInfo, ServerType};
use common::net::{InnerSender, LenBuffer, MsgIO, ALPN_PRIM};
use common::util::timestamp;
use common::Result;
use delay_timer::prelude::{DelayTimer, DelayTimerBuilder, TaskBuilder};
use futures_util::StreamExt;
use quinn::NewConnection;
use tracing::{debug, error, info};

pub(crate) struct Server {}

impl Server {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![CONFIG.server.cert.clone()], CONFIG.server.key.clone())?;
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        quinn_server_config.concurrent_connections(CONFIG.server.max_connections);
        quinn_server_config.use_retry(true);
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(CONFIG.transport.max_bi_streams)
            .max_concurrent_uni_streams(CONFIG.transport.max_uni_streams)
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                CONFIG.transport.connection_idle_timeout,
            )));
        let (endpoint, mut incoming) =
            quinn::Endpoint::server(quinn_server_config, CONFIG.server.outer_address)?;
        let timer = DelayTimerBuilder::default()
            .tokio_runtime_by_default()
            .build();
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let timer = timer.clone();
            tokio::spawn(async move {
                let res = handle_new_connection(conn, timer).await;
                if let Err(e) = res {
                    error!("error handling new connection: {}", e);
                }
            });
        }
        endpoint.wait_idle().await;
        Ok(())
    }
}

pub(crate) async fn handle_new_connection(
    mut conn: NewConnection,
    mut timer: DelayTimer,
) -> Result<()> {
    let io_streams = conn.bi_streams.next().await;
    if io_streams.is_none() {
        return Err(anyhow!("no io streams"));
    }
    let (mut send, mut recv) = io_streams.unwrap()?;
    let mut buffer: Box<LenBuffer> = Box::new([0_u8; 4]);
    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await?;
    let server_info = ServerInfo::from(msg.payload());
    match server_info.typ {
        ServerType::ReplayNode => {}
        _ => {
            return Err(anyhow!("invalid server type"));
        }
    }
    let ack_msg = msg.generate_ack(msg.timestamp());
    MsgIO::write_msg(Arc::new(ack_msg), &mut send).await?;
    let mut failed_send = conn.connection.open_uni().await?;
    let mut failed_channel =
        tokio::sync::mpsc::channel(CONFIG.performance.max_sender_side_channel_size);
    tokio::spawn(async move {
        loop {
            let msg = failed_channel.1.recv().await;
            if let Some(msg) = msg {
                let res = MsgIO::write_msg(msg, &mut failed_send).await;
                if res.is_err() {
                    error!("error writing msg to informer");
                    break;
                }
            } else {
                error!("send channel closed");
                break;
            }
        }
    });
    let ack_map = get_ack_map();
    let cluster_set = get_cluster_connection_set();
    tokio::spawn(async move {
        loop {
            let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
            if let Ok(msg) = msg {
                let res = msg_handler(
                    msg.clone(),
                    &ack_map,
                    &failed_channel.0,
                    &mut timer,
                    &cluster_set,
                )
                .await;
                if res.is_err() {
                    error!("error handling msg: {}", res.unwrap_err());
                }
                let ack_msg = msg.generate_ack(msg.timestamp());
                let res = MsgIO::write_msg(Arc::new(ack_msg), &mut send).await;
                if res.is_err() {
                    error!("error writing ack msg");
                    break;
                }
            } else {
                error!("error reading msg from informer");
                break;
            }
        }
    });
    Ok(())
}

pub(self) async fn msg_handler(
    msg: Arc<Msg>,
    ack_map: &AckMap,
    failed_sender: &InnerSender,
    timer: &mut DelayTimer,
    cluster_set: &ClusterConnectionSet,
) -> Result<()> {
    let mode_value = String::from_utf8_lossy(msg.extension()).parse::<u8>()?;
    let mode = ReplayMode::from(mode_value);
    let replay_id = String::from_utf8_lossy(msg.payload()).to_string();
    match mode {
        ReplayMode::Origin => {
            ack_map.insert(replay_id);
            let failed_sender = failed_sender.clone();
            let task = TaskBuilder::default()
                .set_task_id(timestamp())
                .set_frequency_once_by_seconds(CONFIG.max_deal_time.as_secs())
                .set_maximum_parallel_runnable_num(1)
                .spawn_async_routine(move || {
                    let msg = msg.clone();
                    let sender = failed_sender.clone();
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
            Ok(())
        }
        ReplayMode::Target => {
            let res = ack_map.remove(&replay_id);
            if res.is_some() {
                return Ok(());
            }
            debug!("replay_id: {} may not be here.", replay_id);
            let mut msg = (*msg).clone();
            let vec = vec![ReplayMode::Target.value()];
            msg.set_extension(vec.as_slice());
            let msg = Arc::new(msg);
            for conn in cluster_set.iter() {
                let sender = conn.value();
                let msg = msg.clone();
                let res = sender.send(msg).await;
                if let Err(e) = res {
                    error!("error sending msg to cluster: {}", e);
                }
            }
            Ok(())
        }
        _ => Err(anyhow!("invalid replay mode")),
    }
}
