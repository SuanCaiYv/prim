mod client;
mod server;

use common::Result;
use tracing::error;

pub(super) async fn start() -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = server::Server::new().run().await {
            error!("error running server: {}", e);
        }
    });
    tokio::spawn(async move {
        if let Err(e) = client::Cluster::run().await {
            error!("error running client: {}", e);
        }
    });
    Ok(())
}
