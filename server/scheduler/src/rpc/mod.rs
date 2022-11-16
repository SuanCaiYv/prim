mod node;
mod node_proto;

use std::env;

use lib::Result;

#[allow(unused)]
pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./src/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./src/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}

pub(crate) async fn start() -> Result<()> {
    node::NodeServer::run().await?;
    Ok(())
}
