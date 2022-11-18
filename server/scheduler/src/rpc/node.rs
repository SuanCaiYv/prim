use async_trait::async_trait;
use lib::Result;
use tonic::{transport::{ServerTlsConfig, Server}, Request, Response, Status};

use crate::{config::CONFIG, business::which_node};

use super::node_proto::{user_node_server::{UserNodeServer, UserNode}, UserNodeRequest, UserNodeResponse};

pub(crate) struct NodeServer {}

impl NodeServer {
    pub(crate) async fn run() -> Result<()> {
        let identity =
            tonic::transport::Identity::from_pem(CONFIG.rpc.cert.clone(), CONFIG.rpc.key.clone());
        let server = NodeServer {};
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity))?
            .add_service(UserNodeServer::new(server))
            .serve(CONFIG.rpc.address)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl UserNode for NodeServer {
    async fn which_node(
        &self,
        request: Request<UserNodeRequest>,
    ) -> std::result::Result<Response<UserNodeResponse>, Status> {
        let node_id = which_node(request.into_inner().user_id).await;
        match node_id {
            Ok(node_id) => Ok(Response::new(UserNodeResponse { node_id })),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}
