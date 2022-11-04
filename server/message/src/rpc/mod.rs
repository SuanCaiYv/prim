mod node_proto;
mod node;

use std::env;
use tokio::sync::OnceCell;
use common::Result;
use crate::rpc::node::NodeClient;

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
pub(crate) static NODE_CLIENT: OnceCell<NodeClient> = OnceCell::const_new();

#[allow(unused)]
pub(super) async fn get_node_client() -> NodeClient {
    (NODE_CLIENT
        .get_or_init(|| async { NodeClient::new().await.unwrap() })
        .await)
        .clone()
}