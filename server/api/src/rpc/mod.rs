mod node;
mod node_proto;

use crate::rpc::node::NodeClient;
use common::Result;
use std::env;
use tokio::sync::OnceCell;

pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./src/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./src/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}

pub(crate) static NODE_CLIENT: OnceCell<NodeClient> = OnceCell::const_new();

pub(super) async fn get_node_client() -> NodeClient {
    (NODE_CLIENT
        .get_or_init(|| async { NodeClient::new().await.unwrap() })
        .await)
        .clone()
}
