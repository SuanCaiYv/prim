mod node_proto;
mod node;

use std::env;
use std::path::PathBuf;
use common::Result;

pub(crate) fn gen() -> Result<()> {
    env::set_var("OUT_DIR", "./src/rpc");
    tonic_build::configure()
        .type_attribute("node_proto.UserNodeRequest", "#[derive(Hash)]")
        .type_attribute("node_proto.UserNodeResponse", "#[derive(Hash)]")
        .compile(&["./src/rpc/proto/node.proto"], &["proto"])?;
    Ok(())
}