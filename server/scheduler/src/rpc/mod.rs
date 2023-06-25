use std::env;

use lib_tokio::Result;
use tokio::sync::OnceCell;

use self::node::RpcClient;

mod node;
mod node_proto;

#[allow(unused)]
pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./scheduler/src/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./scheduler/src/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}

#[allow(unused)]
pub(crate) static NODE_CLIENT: OnceCell<RpcClient> = OnceCell::const_new();

#[allow(unused)]
pub(crate) async fn get_rpc_client() -> RpcClient {
    (NODE_CLIENT
        .get_or_init(|| async { RpcClient::new().await.unwrap() })
        .await)
        .clone()
}

pub(crate) async fn start() -> Result<()> {
    node::RpcServer::run().await
}
