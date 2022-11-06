mod node;
mod node_proto;

use crate::rpc::node::Client;
use common::Result;
use std::env;
use tokio::sync::OnceCell;

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
pub(super) async fn get_rpc_client() -> Client {
    (NODE_CLIENT
        .get_or_init(|| async { Client::new().await.unwrap() })
        .await)
        .clone()
}
