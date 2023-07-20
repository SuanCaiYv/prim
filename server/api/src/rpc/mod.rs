use lib::Result;
use tokio::sync::OnceCell;

use self::node::Client;

pub(crate) mod node;
mod node_proto;

#[allow(unused)]
pub(crate) static NODE_CLIENT: OnceCell<Client> = OnceCell::const_new();

#[allow(unused)]
pub(crate) async fn get_rpc_client() -> Client {
    (NODE_CLIENT
        .get_or_init(|| async { Client::new().await.unwrap() })
        .await)
        .clone()
}

pub(crate) async fn start() -> Result<()> {
    node::RpcServer::run().await
}
