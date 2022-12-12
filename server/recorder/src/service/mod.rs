use anyhow::anyhow;
use lazy_static::lazy_static;
use lib::{
    net::{InnerSender, OuterReceiver},
    Result,
};
use tokio::sync::RwLock;
use tracing::error;

use crate::model::msg::Message;

pub(self) mod handler;
pub(self) mod server;

pub(crate) static BUFFER_SIZE: usize = 256 * 4;

lazy_static! {
    static ref BUFFER_CHANNEL: (InnerSender, RwLock<Option<OuterReceiver>>) = get_buffer_channel();
}

pub(self) fn get_buffer_channel() -> (InnerSender, RwLock<Option<OuterReceiver>>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(BUFFER_SIZE);
    (sender, RwLock::new(Some(receiver)))
}

pub(self) async fn io_loop() -> Result<()> {
    let mut buf = Vec::with_capacity(BUFFER_SIZE / 2);
    let mut index: usize = 0;
    let mut buffer_receiver;
    {
        buffer_receiver = BUFFER_CHANNEL.1.write().await.take().unwrap();
    }
    loop {
        match buffer_receiver.recv().await {
            Some(msg) => {
                if index == buf.len() {
                    Message::insert_batch(&buf).await?;
                    index = 0;
                }
                buf[index] = Message::from(&(*msg));
                index += 1;
            }
            None => {
                return Err(anyhow!("buffer channel closed."));
            }
        }
    }
}

pub(crate) async fn start() -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = io_loop().await {
            error!("io loop error: {}", e);
        }
    });
    server::Server::run().await?;
    Ok(())
}
