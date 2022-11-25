use lib::Result;

use rand::random;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};
use anyhow::anyhow;

use crate::{config::CONFIG, util::my_id};

use super::node_proto::{
    api_client::ApiClient, scheduler_client::SchedulerClient, CurrNodeGroupIdUserListReq, AddGroupReq,
};

#[derive(Clone)]
pub(crate) struct Client {
    scheduler_client: SchedulerClient<Channel>,
    #[allow(unused)]
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
        let scheduler_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let scheduler_client = SchedulerClient::new(scheduler_channel);
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
            scheduler_client,
            api_client,
        })
    }

    #[allow(unused)]
    pub(crate) async fn call_curr_node_group_id_user_list(
        &mut self,
        group_id: u64,
    ) -> Result<Vec<u64>> {
        let request = Request::new(CurrNodeGroupIdUserListReq {
            node_id: my_id(),
            group_id,
        });
        let response = self
            .scheduler_client
            .curr_node_group_id_user_list(request)
            .await?;
        Ok(response.into_inner().user_id_list)
    }

    #[allow(unused)]
    pub(crate) async fn call_scheduler_add_group(&mut self, group_id: u64, user_id: u64) -> Result<()> {
        let request = Request::new(AddGroupReq {
            group_id,
            user_id,
        });
        let response = self.scheduler_client.add_group(request).await?;
        let inner = response.into_inner();
        if inner.success {
            Ok(())
        } else {
            Err(anyhow!(inner.err_msg))
        }
    }
}
