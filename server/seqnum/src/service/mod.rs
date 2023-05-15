use lib::Result;

pub(crate) mod handler;
pub(crate) mod server;

pub(crate) async fn start() -> Result<()> {
    server::Server::run().await
}
