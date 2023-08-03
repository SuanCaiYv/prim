use lib::{net::GenericParameter, Result};
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};

use super::node_proto::{
    api_client::ApiClient, scheduler_client::SchedulerClient, AllGroupNodeListReq,
    CurrNodeGroupIdUserListReq, SeqnumAllNodeReq, SeqnumNodeUserSelectReq, SeqnumNodeAddressReq,
};
use crate::{config::config, util::my_id};

#[derive(Clone)]
pub(crate) struct RpcClient {
    scheduler_client: SchedulerClient<Channel>,
    #[allow(unused)]
    api_client: ApiClient<Channel>,
}

impl RpcClient {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(config().rpc.scheduler.cert.clone())
            .domain_name(config().rpc.scheduler.domain.clone());
        let host = format!("https://{}", config().rpc.scheduler.address).to_string();
        let scheduler_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        println!("init1");
        let scheduler_client = SchedulerClient::new(scheduler_channel);
        let tls = ClientTlsConfig::new()
            .ca_certificate(config().rpc.api.cert.clone())
            .domain_name(config().rpc.api.domain.clone());
        let host = format!("https://{}", config().rpc.api.address).to_string();
        let api_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        println!("init2");
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
        Ok(response.into_inner().user_list)
    }

    pub(crate) async fn call_all_group_node_list(&mut self, group_id: u64) -> Result<Vec<u32>> {
        let request = Request::new(AllGroupNodeListReq { group_id });
        let response = self.scheduler_client.all_group_node_list(request).await?;
        Ok(response.into_inner().node_list)
    }

    pub(crate) async fn call_seqnum_all_node(&mut self) -> Result<(Vec<u32>, Vec<String>)> {
        let request = Request::new(SeqnumAllNodeReq {
            node_id: 0,
        });
        let response = self.scheduler_client.seqnum_all_node(request).await?;
        let inner = response.into_inner();
        Ok((inner.node_id_list, inner.address_list))
    }

    pub(crate) async fn call_seqnum_node_user_select(&mut self, key: u128) -> Result<u32> {
        let request = Request::new(SeqnumNodeUserSelectReq {
            user_id1: (key >> 64) as u64,
            user_id2: key as u64,
        });
        let response = self.scheduler_client.seqnum_node_user_select(request).await?;
        Ok(response.into_inner().node_id)
    }

    pub(crate) async fn call_seqnum_node_address(&mut self, node_id: u32) -> Result<String> {
        let request = Request::new(SeqnumNodeAddressReq { node_id });
        let response = self.scheduler_client.seqnum_node_address(request).await?;
        Ok(response.into_inner().address)
    }
}

impl GenericParameter for RpcClient {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
