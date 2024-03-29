use async_trait::async_trait;
use base64::Engine;
use lib::{entity::Msg, Result};
use tonic::{
    transport::{Channel, ClientTlsConfig, Server, ServerTlsConfig},
    Request, Response, Status,
};
use tracing::{error, info};

use super::node_proto::{
    api_server::{Api, ApiServer},
    scheduler_client::SchedulerClient,
    GroupUserListReq, GroupUserListResp, PushMsgReq, WhichNodeReq,
};
use crate::rpc::node_proto::WhichToConnectReq;
use crate::{config::config, model::group::Group};

#[derive(Clone)]
pub(crate) struct Client {
    scheduler_client: SchedulerClient<Channel>,
}

impl Client {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(config().rpc.scheduler.cert.clone())
            .domain_name(config().rpc.scheduler.domain.clone());
        let host = format!("https://{}", config().rpc.scheduler.address).to_string();
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
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        let request = Request::new(PushMsgReq {
            sender: msg.sender(),
            receiver: msg.receiver(),
            timestamp: msg.timestamp(),
            version: msg.version(),
            r#type: msg.typ().value() as u32,
            payload: engine.encode(msg.payload()),
            extension: engine.encode(msg.extension()),
        });
        let response = self.scheduler_client.push_msg(request).await?;
        let resp = response.into_inner();
        if resp.success {
            Ok(())
        } else {
            return Err(anyhow::anyhow!(resp.err_msg));
        }
    }

    pub(crate) async fn call_which_to_connect(&mut self, user_id: u64) -> Result<String> {
        let request = Request::new(WhichToConnectReq { user_id });
        let response = self.scheduler_client.which_to_connect(request).await?;
        Ok(response.into_inner().address)
    }
}

pub(crate) struct RpcServer {}

impl RpcServer {
    pub(crate) async fn run() -> Result<()> {
        let identity =
            tonic::transport::Identity::from_pem(config().rpc.cert.clone(), config().rpc.key.clone());
        let server = RpcServer {};
        info!("rpc server running on {}", config().rpc.address);
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity))?
            .add_service(ApiServer::new(server))
            .serve(config().rpc.address)
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
        let res = match Group::get_group_id(group_id as i64).await {
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
            Err(e) => {
                error!("get group by group_id error: {}", e);
                Err(Status::internal(e.to_string()))
            }
        };
        info!("group_user_list: {:?}", res);
        res
    }
}
