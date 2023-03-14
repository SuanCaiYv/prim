use async_trait::async_trait;
use lib::{entity::Msg, Result};
use tonic::{
    transport::{Channel, ClientTlsConfig, Server, ServerTlsConfig},
    Request, Response, Status,
};
use tracing::info;

use super::node_proto::{
    api_server::{Api, ApiServer},
    scheduler_client::SchedulerClient,
    GroupUserListReq, GroupUserListResp, PushMsgReq, WhichNodeReq,
};
use crate::{config::CONFIG, model::group::Group};
use crate::rpc::node_proto::WhichToConnectReq;

#[derive(Clone)]
pub(crate) struct Client {
    scheduler_client: SchedulerClient<Channel>,
}

impl Client {
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
    pub(crate) async fn call_which_node(&mut self, user_id: u64) -> Result<u32> {
        let request = Request::new(WhichNodeReq { user_id });
        let response = self.scheduler_client.which_node(request).await?;
        Ok(response.into_inner().node_id)
    }

    #[allow(unused)]
    pub(crate) async fn call_push_msg(&mut self, msg: &Msg) -> Result<()> {
        let request = Request::new(PushMsgReq {
            sender: msg.sender(),
            receiver: msg.receiver(),
            timestamp: msg.timestamp(),
            version: msg.version(),
            r#type: msg.typ().value() as u32,
            payload: base64::encode_config(msg.payload(), base64::URL_SAFE),
            extension: base64::encode_config(msg.extension(), base64::URL_SAFE),
        });
        let response = self.scheduler_client.push_msg(request).await?;
        let resp = response.into_inner();
        if resp.success {
            Ok(())
        } else {
            return Err(anyhow::anyhow!(resp.err_msg));
        }
    }

    pub (crate) async fn call_which_to_connect(&mut self, user_id: u64) -> Result<String> {
        let request = Request::new(WhichToConnectReq { user_id });
        let response = self.scheduler_client.which_to_connect(request).await?;
        Ok(response.into_inner().address)
    }
}

pub(crate) struct RpcServer {}

impl RpcServer {
    pub(crate) async fn run() -> Result<()> {
        let identity =
            tonic::transport::Identity::from_pem(CONFIG.rpc.cert.clone(), CONFIG.rpc.key.clone());
        let server = RpcServer {};
        info!("rpc server running on {}", CONFIG.rpc.address);
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity))?
            .add_service(ApiServer::new(server))
            .serve(CONFIG.rpc.address)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Api for RpcServer {
    async fn group_user_list(
        &self,
        request: Request<GroupUserListReq>,
    ) -> std::result::Result<Response<GroupUserListResp>, Status> {
        let request_inner = request.into_inner();
        let group_id = request_inner.group_id;
        match Group::get_group_id(group_id as i64).await {
            Ok(group) => {
                let mut user_list = vec![];
                for user in group.member_list.iter() {
                    if let Some(map) = user.as_object() {
                        let user_id = map.get("user_id").unwrap();
                        let user_id = user_id.as_i64().unwrap() as u64;
                        user_list.push(user_id);
                    }
                }
                for user in group.admin_list.iter() {
                    if let Some(map) = user.as_object() {
                        let user_id = map.get("user_id").unwrap();
                        let user_id = user_id.as_i64().unwrap() as u64;
                        user_list.push(user_id);
                    }
                }
                Ok(Response::new(GroupUserListResp { user_list }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
