pub(crate) mod cluster;
mod inner;
mod outer;
pub(super) mod server;

use crate::config::CONFIG;
use common::Result;
use tracing::error;

pub(crate) async fn start() -> Result<()> {
    let (sender, receiver) =
        tokio::sync::mpsc::channel(CONFIG.performance.max_receiver_side_channel_size);
    tokio::spawn(async move {
        let res = outer::start(sender).await;
        if let Err(e) = res {
            error!("outer start error: {}", e);
        }
    });
    tokio::spawn(async move {
        let res = inner::start(receiver).await;
        if let Err(e) = res {
            error!("inner start error: {}", e);
        }
    });
    Ok(())
}
