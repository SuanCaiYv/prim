use crate::config::CONFIG;
use crate::outer::rpc::node_proto::user_node_server::{UserNode, UserNodeServer};
use crate::outer::rpc::node_proto::{UserNodeRequest, UserNodeResponse};
use async_trait::async_trait;
use common::Result;
use tonic::transport::{Server, ServerTlsConfig};
use tonic::{Request, Response, Status};
use tracing::info;

pub(crate) struct NodeServer {}

impl NodeServer {
    pub(crate) async fn run() -> Result<()> {
        let identity =
            tonic::transport::Identity::from_pem(CONFIG.rpc.cert.clone(), CONFIG.rpc.key.clone());
        let server = NodeServer {};
        info!("RPC NodeServer is running on {}", CONFIG.rpc.address);
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
        let node_id = crate::outer::which_node(request.into_inner().user_id).await;
        match node_id {
            Ok(node_id) => Ok(Response::new(UserNodeResponse { node_id })),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}
