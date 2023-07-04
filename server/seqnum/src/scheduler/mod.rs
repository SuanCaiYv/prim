use lib::Result;
use lib_net_monoio::net::ReqwestOperatorManager;

pub(self) mod client;

pub(crate) async fn start() -> Result<ReqwestOperatorManager> {
    client::Client::run().await
}