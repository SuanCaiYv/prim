mod server;
pub mod client;

use common::Result;

pub(super) async fn start() -> Result<()> {
    server::Server::new().run().await?;
    Ok(())
}