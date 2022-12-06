use lib::Result;

pub(self) mod handler;
pub(self) mod server;

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await?;
    Ok(())
}
