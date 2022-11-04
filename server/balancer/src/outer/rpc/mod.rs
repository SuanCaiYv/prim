mod node;
mod node_proto;

use common::Result;
use std::env;

#[allow(unused)]
pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./src/outer/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./src/outer/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}

pub(crate) async fn start() -> Result<()> {
    node::NodeServer::run().await?;
    Ok(())
}
