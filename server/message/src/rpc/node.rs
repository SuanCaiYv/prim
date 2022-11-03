use super::node_proto::user_node_client::UserNodeClient;
use crate::config::CONFIG;
use crate::rpc::node_proto::UserNodeRequest;
use common::Result;
use rand::random;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};

pub(crate) struct NodeClient {
    inner: UserNodeClient<Channel>,
}

impl NodeClient {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(CONFIG.rpc.cert.clone())
            .domain_name(CONFIG.rpc.domain.clone());
        let index: u8 = random();
        let index = index as usize % CONFIG.rpc.addresses.len();
        let host = format!("https://{}", CONFIG.rpc.addresses[index]).to_string();
        let channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let client = UserNodeClient::new(channel);
        Ok(Self { inner: client })
    }

    pub(crate) async fn call_which_node(&mut self, user_id: u64) -> Result<u32> {
        let request = Request::new(UserNodeRequest { user_id });
        let response = self.inner.which_node(request).await;
        match response {
            Ok(response) => {
                let response = response.into_inner();
                Ok(response.node_id)
            }
            Err(err) => Err(err.into()),
        }
    }
}
