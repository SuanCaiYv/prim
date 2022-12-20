use std::net::SocketAddr;

use lib::Result;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};

use super::node_proto::{scheduler_client::SchedulerClient, RecorderListReq};
use crate::config::CONFIG;

#[derive(Clone)]
pub(crate) struct RpcClient {
    scheduler_client: SchedulerClient<Channel>,
}

impl RpcClient {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(CONFIG.rpc.scheduler.cert.clone())
            .domain_name(CONFIG.rpc.scheduler.domain.clone());
        let index: u8 = fastrand::u8(..);
        let index = index as usize % CONFIG.rpc.scheduler.addresses.len();
        let host = format!("https://{}", CONFIG.rpc.scheduler.addresses[index]).to_string();
        let scheduler_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let scheduler_client = SchedulerClient::new(scheduler_channel);
        Ok(Self { scheduler_client })
    }

    #[allow(unused)]
    pub(crate) async fn call_recorder_list(
        &mut self,
        group_id: u64,
    ) -> Result<(Vec<u32>, Vec<SocketAddr>)> {
        let request = Request::new(RecorderListReq {});
        let response = self.scheduler_client.recorder_list(request).await?;
        let resp = response.into_inner();
        let addresses = resp
            .address_list
            .iter()
            .map(|x| x.parse().expect("parse address error"))
            .collect::<Vec<SocketAddr>>();
        Ok((resp.node_id_list, addresses))
    }
}
