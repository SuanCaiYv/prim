mod client;
mod handler;

use lib::Result;

pub(crate) async fn start() -> Result<()> {
    client::Client::run().await?;
    Ok(())
}
