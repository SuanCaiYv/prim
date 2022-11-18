use lib::Result;
use rand::random;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};

use crate::config::CONFIG;

use super::node_proto::{
    api_client::ApiClient, balancer_client::BalancerClient, UserGroupListRequest,
    UserNodeMapRequest,
};

#[derive(Clone)]
pub(crate) struct Client {
    balancer_client: BalancerClient<Channel>,
    api_client: ApiClient<Channel>,
}

impl Client {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(CONFIG.rpc.scheduler.cert.clone())
            .domain_name(CONFIG.rpc.scheduler.domain.clone());
        let index: u8 = random();
        let index = index as usize % CONFIG.rpc.scheduler.addresses.len();
        let host = format!("https://{}", CONFIG.rpc.scheduler.addresses[index]).to_string();
        let balancer_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let balancer_client = BalancerClient::new(balancer_channel);
        let tls = ClientTlsConfig::new()
            .ca_certificate(CONFIG.rpc.api.cert.clone())
            .domain_name(CONFIG.rpc.api.domain.clone());
        let index: u8 = random();
        let index = index as usize % CONFIG.rpc.api.addresses.len();
        let host = format!("https://{}", CONFIG.rpc.api.addresses[index]).to_string();
        let api_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let api_client = ApiClient::new(api_channel);
        Ok(Self {
            balancer_client,
            api_client,
        })
    }

    #[allow(unused)]
    pub(crate) async fn call_which_node(&mut self, user_id: u64) -> Result<u32> {
        let request = Request::new(UserNodeMapRequest { user_id });
        let response = self.balancer_client.which_node(request).await;
        match response {
            Ok(response) => {
                let response = response.into_inner();
                Ok(response.node_id)
            }
            Err(err) => Err(err.into()),
        }
    }

    #[allow(unused)]
    pub(crate) async fn call_user_group_list(&mut self, user_id: u64) -> Result<Vec<u64>> {
        let request = Request::new(UserGroupListRequest { user_id });
        let response = self.api_client.user_group_list(request).await;
        match response {
            Ok(response) => {
                let response = response.into_inner();
                Ok(response.group_id_list)
            }
            Err(err) => Err(err.into()),
        }
    }
}
