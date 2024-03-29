use async_trait::async_trait;
use base64::Engine;
use lib::{
    entity::{Msg, ReqwestMsg, ReqwestResourceID, Type},
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
        AllGroupNodeListReq, AllGroupNodeListResp, CurrNodeGroupIdUserListReq,
        CurrNodeGroupIdUserListResp, GroupUserListReq, MessageNodeAliveReq, MessageNodeAliveResp,
        PushMsgReq, PushMsgResp, SeqnumAllNodeReq, SeqnumAllNodeResp, SeqnumNodeAddressReq,
        SeqnumNodeAddressResp, SeqnumNodeUserSelectReq, SeqnumNodeUserSelectResp, WhichNodeReq,
        WhichNodeResp,
    },
};
use crate::{
    cache::{get_redis_ops, USER_NODE_MAP},
    config::config,
    service::{get_client_caller_map, get_message_node_set, get_server_info_map},
};
use crate::{
    rpc::node_proto::{WhichToConnectReq, WhichToConnectResp},
    service::get_seqnum_node_set,
};

#[derive(Clone)]
pub(crate) struct RpcClient {
    #[allow(unused)]
    api_client: ApiClient<Channel>,
}

impl RpcClient {
    pub(crate) async fn new() -> Result<Self> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(config().rpc.api.cert.clone())
            .domain_name(config().rpc.api.domain.clone());
        let host = format!("https://{}", config().rpc.api.address).to_string();
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
        let identity = tonic::transport::Identity::from_pem(&config().rpc.cert, &config().rpc.key);
        let server = RpcServer {};
        info!("rpc server running on {}", config().rpc.address);
        let server_config = ServerTlsConfig::new().identity(identity);
        Server::builder()
            .tls_config(server_config)
            .unwrap()
            .add_service(SchedulerServer::new(server))
            .serve(config().rpc.address)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Scheduler for RpcServer {
    // todo, change implement to use redis record map relationship.
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
            let key = format!("{}{}", USER_NODE_MAP, user_id);
            let node_id = match redis_ops.get::<u32>(&key).await {
                Ok(node_id) => node_id,
                // todo: if user not in redis, we should add it.
                Err(_) => {
                    let node_id = self
                        .which_node(Request::new(WhichNodeReq { user_id: *user_id }))
                        .await?;
                    let node_id = node_id.into_inner().node_id;
                    _ = redis_ops.set(&key, &node_id).await;
                    node_id
                }
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
        request: Request<WhichNodeReq>,
    ) -> std::result::Result<Response<WhichNodeResp>, Status> {
        let user_id = request.into_inner().user_id;
        let key = format!("{}{}", USER_NODE_MAP, user_id);
        // todo unsafecell optimization.
        let mut redis_ops = get_redis_ops().await;
        let set = get_message_node_set().0;
        let value: Result<u32> = redis_ops.get(&key).await;
        let node_id = match value {
            Ok(value) => value,
            Err(_) => {
                let node_size = set.len();
                if node_size == 0 {
                    return Err(Status::internal("message cluster all crashed."));
                }
                let index = user_id % (node_size as u64);
                let node_id = *match set.iter().nth(index as usize) {
                    Some(v) => v,
                    None => return Err(Status::internal("try again.")),
                };
                match redis_ops.set(&key, &node_id).await {
                    Ok(_) => node_id as u32,
                    Err(_) => {
                        return Err(Status::internal("redis set error"));
                    }
                }
            }
        };
        Ok(Response::new(WhichNodeResp { node_id }))
    }

    async fn all_group_node_list(
        &self,
        _request: Request<AllGroupNodeListReq>,
    ) -> std::result::Result<Response<AllGroupNodeListResp>, Status> {
        // todo, change implement to use redis recorded map relationship.
        let list = get_message_node_set().0.iter().map(|v| *v as u32).collect();
        Ok(Response::new(AllGroupNodeListResp { node_list: list }))
    }

    /// this method will only forward the msg to corresponding node.
    async fn push_msg(
        &self,
        request: tonic::Request<PushMsgReq>,
    ) -> std::result::Result<Response<PushMsgResp>, Status> {
        let req = request.into_inner();
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::URL_SAFE,
            base64::engine::general_purpose::NO_PAD,
        );
        let payload = engine.decode(req.payload);
        let payload = match payload {
            Ok(payload) => payload,
            Err(_) => {
                return Err(Status::internal("base64 decode error"));
            }
        };
        let extension = engine.decode(req.extension);
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
        let req =
            ReqwestMsg::with_resource_id_payload(ReqwestResourceID::MessageForward, msg.as_slice());
        let client_map = get_client_caller_map().0;
        let sender = client_map.get(&node_id);
        match sender {
            Some(client) => match client.call(req).await {
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

    async fn which_to_connect(
        &self,
        request: Request<WhichToConnectReq>,
    ) -> std::result::Result<Response<WhichToConnectResp>, Status> {
        let recorder_node_set = get_message_node_set().0;
        let index = (request.into_inner().user_id % (recorder_node_set.len() as u64)) as usize;
        let node_id;
        {
            let id = match recorder_node_set.iter().nth(index) {
                Some(node_id) => node_id,
                None => return Err(Status::internal("try again")),
            };
            node_id = *id;
        }
        let node_info_map = get_server_info_map().0;
        let node_info = match node_info_map.get(&node_id) {
            Some(node_info) => node_info,
            None => return Err(Status::internal("node info not found")),
        };
        let address = node_info.service_address.clone();
        Ok(Response::new(WhichToConnectResp { address }))
    }

    async fn seqnum_node_address(
        &self,
        request: Request<SeqnumNodeAddressReq>,
    ) -> std::result::Result<Response<SeqnumNodeAddressResp>, Status> {
        let inner = request.into_inner();
        let server_map = get_server_info_map().0;
        let node_info = match server_map.get(&inner.node_id) {
            Some(node_info) => node_info,
            None => return Err(Status::internal("node info not found")),
        };
        let address = node_info.service_address.clone();
        Ok(Response::new(SeqnumNodeAddressResp {
            node_id: inner.node_id,
            address,
        }))
    }

    async fn seqnum_node_user_select(
        &self,
        request: tonic::Request<SeqnumNodeUserSelectReq>,
    ) -> std::result::Result<Response<SeqnumNodeUserSelectResp>, Status> {
        let inner = request.into_inner();
        let seqnum_set = get_seqnum_node_set().0;
        let key = (inner.user_id1 as u128) << 64 | inner.user_id2 as u128;
        let index = key % (seqnum_set.len() as u128);
        let node_id = match seqnum_set.iter().nth(index as usize) {
            Some(node_id) => *node_id,
            None => return Err(Status::internal("try again")),
        };
        Ok(Response::new(SeqnumNodeUserSelectResp { node_id }))
    }

    async fn seqnum_all_node(
        &self,
        _request: Request<SeqnumAllNodeReq>,
    ) -> std::result::Result<Response<SeqnumAllNodeResp>, Status> {
        let seqnum_set = get_seqnum_node_set().0;
        let server_info_map = get_server_info_map().0;
        let mut node_id_list = Vec::new();
        let mut address_list = Vec::new();
        for node_id in seqnum_set.iter() {
            let node_info = match server_info_map.get(&*node_id) {
                Some(node_info) => node_info,
                None => return Err(Status::internal("node info not found")),
            };
            node_id_list.push(*node_id);
            address_list.push(node_info.service_address.to_string());
        }
        Ok(Response::new(SeqnumAllNodeResp {
            node_id_list,
            address_list,
        }))
    }

    async fn message_node_alive(
        &self,
        _request: Request<MessageNodeAliveReq>,
    ) -> std::result::Result<Response<MessageNodeAliveResp>, Status> {
        todo!("message node alive")
    }
}
