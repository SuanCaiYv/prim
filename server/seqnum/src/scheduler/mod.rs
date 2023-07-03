use lib::Result;

pub(self) mod client;
pub(self) mod handler;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await
}