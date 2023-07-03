use lib::Result;

pub(self) mod client;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await
}