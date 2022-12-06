use std::env;

use lib::Result;
use tokio::sync::OnceCell;

use self::node::Client;

mod node;
mod node_proto;

#[allow(unused)]
pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./src/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./src/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}

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
