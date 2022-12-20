use std::sync::Arc;

use async_trait::async_trait;
use lib::{
    entity::{Msg, Type},
    Result,
};

use tonic::{
    transport::{Channel, ClientTlsConfig, Server, ServerTlsConfig},
    Request, Response, Status,
};
use tracing::info;

use super::{
    get_rpc_client,
    node_proto::{
        api_client::ApiClient,
        scheduler_server::{Scheduler, SchedulerServer},
        CurrNodeGroupIdUserListReq, CurrNodeGroupIdUserListResp, GroupUserListReq, PushMsgReq,
        PushMsgResp, RecorderListReq, RecorderListResp, WhichNodeReq, WhichNodeResp,
    },
};
use crate::{
    cache::{get_redis_ops, USER_NODE_MAP},
    config::CONFIG,
    service::{
        get_client_connection_map, get_message_node_set, get_recorder_node_set, get_server_info_map,
    },
};

#[derive(Clone)]
pub(crate) struct RpcClient {
    #[allow(unused)]
    api_client: ApiClient<Channel>,
}

impl RpcClient {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(CONFIG.rpc.api.cert.clone())
            .domain_name(CONFIG.rpc.api.domain.clone());
        let index: u8 = fastrand::u8(..);
        let index = index as usize % CONFIG.rpc.api.addresses.len();
        let host = format!("https://{}", CONFIG.rpc.api.addresses[index]).to_string();
        let api_channel = Channel::from_shared(host)?
            .tls_config(tls)?
            .connect()
            .await?;
        let api_client = ApiClient::new(api_channel);
        Ok(Self { api_client })
    }

    #[allow(unused)]
    pub(crate) async fn call_group_user_list(&mut self, group_id: u64) -> Result<Vec<u64>> {
        let request = Request::new(GroupUserListReq { group_id });
        let response = self.api_client.group_user_list(request).await?;
        Ok(response.into_inner().user_list)
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
            .tls_config(ServerTlsConfig::new().identity(identity))
            .unwrap()
            .add_service(SchedulerServer::new(server))
            .serve(CONFIG.rpc.address)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Scheduler for RpcServer {
    async fn curr_node_group_id_user_list(
        &self,
        request: Request<CurrNodeGroupIdUserListReq>,
    ) -> std::result::Result<Response<CurrNodeGroupIdUserListResp>, Status> {
        let mut rpc_client = get_rpc_client().await;
        let mut redis_ops = get_redis_ops().await;
        let request_inner = request.into_inner();
        let user_list = match rpc_client
            .call_group_user_list(request_inner.group_id)
            .await
        {
            Ok(user_list) => user_list,
            Err(_) => {
                return Err(Status::internal("call group user list failed"));
            }
        };
        let mut list = vec![];
        for user_id in user_list.iter() {
            let node_id = match redis_ops
                .get::<u32>(&format!("{}{}", USER_NODE_MAP, user_id))
                .await
            {
                Ok(node_id) => node_id,
                Err(_) => return Err(Status::internal("get user node id failed")),
            };
            if node_id == request_inner.node_id {
                list.push(*user_id);
            }
        }
        Ok(Response::new(CurrNodeGroupIdUserListResp {
            user_list: list,
        }))
    }

    async fn which_node(
        &self,
        request: tonic::Request<WhichNodeReq>,
    ) -> std::result::Result<tonic::Response<WhichNodeResp>, Status> {
        let user_id = request.into_inner().user_id;
        let key = format!("{}{}", USER_NODE_MAP, user_id);
        // unsafecell optimization.
        let mut redis_ops = get_redis_ops().await;
        let set = get_message_node_set().0;
        let value: Result<u32> = redis_ops.get(&key).await;
        let node_id = match value {
            Ok(value) => value,
            Err(_) => {
                let node_size = set.len();
                let index = user_id % (node_size as u64);
                match redis_ops.set(&key, &index).await {
                    Ok(_) => index as u32,
                    Err(_) => {
                        return Err(Status::internal("redis set error"));
                    }
                }
            }
        };
        Ok(Response::new(WhichNodeResp { node_id }))
    }

    /// this method will only forward the msg to corresponding node.
    async fn push_msg(
        &self,
        request: tonic::Request<PushMsgReq>,
    ) -> std::result::Result<Response<PushMsgResp>, Status> {
        let req = request.into_inner();
        let payload = base64::decode(req.payload);
        let payload = match payload {
            Ok(payload) => payload,
            Err(_) => {
                return Err(Status::internal("base64 decode error"));
            }
        };
        let extension = base64::decode(req.extension);
        let extension = match extension {
            Ok(extension) => extension,
            Err(_) => {
                return Err(Status::internal("base64 decode error"));
            }
        };
        let node_id = self
            .which_node(Request::new(WhichNodeReq {
                user_id: req.receiver,
            }))
            .await?;
        let node_id = node_id.into_inner().node_id;
        let mut msg = Msg::raw2(
            req.sender,
            req.receiver,
            node_id,
            payload.as_slice(),
            extension.as_slice(),
        );
        msg.set_type(Type::from(req.r#type as u16));
        let client_map = get_client_connection_map().0;
        let sender = client_map.get(&node_id);
        match sender {
            Some(client) => match client.send(Arc::new(msg)).await {
                Ok(_) => Ok(Response::new(PushMsgResp {
                    success: true,
                    err_msg: "".to_string(),
                })),
                Err(_) => Ok(Response::new(PushMsgResp {
                    success: false,
                    err_msg: "send msg failed".to_string(),
                })),
            },
            None => Err(Status::internal("node not found")),
        }
    }

    async fn recorder_list(
        &self,
        _request: Request<RecorderListReq>,
    ) -> std::result::Result<Response<RecorderListResp>, Status> {
        let recorder_node_set = get_recorder_node_set().0;
        let mut list = vec![];
        for node_id in recorder_node_set.iter() {
            list.push(*node_id);
        }
        let node_info_map = get_server_info_map().0;
        let mut resp_list = vec![];
        for node_id in list.iter() {
            let node_info = node_info_map.get(node_id);
            match node_info {
                Some(node_info) => {
                    resp_list.push(node_info.address.to_string());
                }
                None => {
                    return Err(Status::internal("node info not found"));
                }
            }
        }
        Ok(Response::new(RecorderListResp {
            address_list: resp_list,
            node_id_list: list,
        }))
    }
}
