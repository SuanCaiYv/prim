use std::sync::Arc;

use super::{MsgMode, CLIENT_SENDER_MAP};
use anyhow::anyhow;
use common::{
    net::{InnerSender, LenBuffer, MsgIO},
    Result,
};
use futures_util::StreamExt;
use quinn::NewConnection;
use tracing::error;

pub(crate) async fn handle_new_connection(
    mut conn: NewConnection,
    global_sender: InnerSender,
) -> Result<()> {
    let io_streams = conn.bi_streams.next().await;
    if io_streams.is_none() {
        return Err(anyhow!("no io streams"));
    }
    let (mut send, mut recv) = io_streams.unwrap()?;
    let mut buffer: Box<LenBuffer> = Box::new([0_u8; 4]);
    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await?;
    let sender = msg.sender_node();
    let mode_value = String::from_utf8_lossy(msg.extension()).parse::<u8>()?;
    let mode = MsgMode::from(mode_value);
    if mode == MsgMode::Cluster {
        tokio::spawn(async move {
            loop {
                let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
                if let Ok(msg) = msg {
                    let res = global_sender.send(msg).await;
                    if res.is_err() {
                        error!("global sender closed");
                        break;
                    }
                } else {
                    error!("error reading msg from cluster");
                    break;
                }
            }
        });
    } else {
        let mut informer = conn.connection.open_uni().await?;
        let mut send_channel = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            loop {
                let msg = send_channel.1.recv().await;
                if let Some(msg) = msg {
                    let res = MsgIO::write_msg(msg, &mut informer).await;
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
        let client_map = CLIENT_SENDER_MAP.clone();
        client_map.insert(sender, send_channel.0);
        tokio::spawn(async move {
            loop {
                let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
                if let Ok(msg) = msg {
                    let res = global_sender.send(msg.clone()).await;
                    if res.is_err() {
                        error!("global sender closed");
                        break;
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
    }
    Ok(())
}
