use std::sync::Arc;

use crate::config::CONFIG;
use crate::core::{get_cluster_connection_map, ClusterConnectionMap};
use crate::util::my_id;
use ahash::AHashMap;
use common::entity::{Msg, ServerInfo, ServerStatus, ServerType, Type};
use common::error::HandlerError;
use common::net::client::{ClientConfigBuilder, ClientMultiConnection, ClientSubConnectionConfig};
use common::net::server::{GenericParameterMap, HandlerList, HandlerParameters};
use common::net::InnerSender;
use common::util::default_bind_ip;
use common::Result;
use tracing::error;

pub(crate) struct Client {
    io_channel_sender: InnerSender,
    handler_list: HandlerList,
    multi_client: ClientMultiConnection,
    cluster_map: ClusterConnectionMap,
}

impl Client {
    pub(crate) async fn new(
        io_channel_sender: InnerSender,
        handler_list: HandlerList,
    ) -> Result<Client> {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(default_bind_ip()) // note: this address is not used
            .with_domain(CONFIG.server.domain.clone())
            .with_cert(CONFIG.server.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let config = client_config.build().unwrap();
        let multi_client = ClientMultiConnection::new(config).await?;
        Ok(Client {
            io_channel_sender,
            handler_list,
            multi_client,
            cluster_map: get_cluster_connection_map(),
        })
    }

    pub(crate) async fn run(&self, sender_server_info: &ServerInfo) -> Result<()> {
        let my_address = CONFIG.server.service_address;
        let m_id = my_id();
        let mut sub_connection = self
            .multi_client
            .new_connection(ClientSubConnectionConfig {
                remote_address: sender_server_info.address,
                domain: CONFIG.server.domain.clone(),
                opened_bi_streams_number: 3,
                opened_uni_streams_number: 3,
            })
            .await?;
        let mut rw_channel = sub_connection.operation_channel().await?;
        let resp_server_info = ServerInfo {
            id: m_id,
            address: my_address,
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MessageCluster,
            load: None,
        };
        let payload = resp_server_info.to_bytes();
        let mut msg = Msg::raw_payload(&payload);
        msg.set_type(Type::Auth);
        msg.set_sender(sender_server_info.id as u64);
        msg.set_sender_node(sender_server_info.id);
        rw_channel.0.send(Arc::new(msg)).await?;
        let _ = self
            .cluster_map
            .insert(sender_server_info.id, rw_channel.0.clone());
        let mut handler_parameters = HandlerParameters {
            io_handler_sender: self.io_channel_sender.clone(),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        let text = format!("hello peer, I am {}", m_id);
        let msg = Msg::text(
            m_id as u64,
            sender_server_info.id as u64,
            m_id,
            sender_server_info.id,
            text,
        );
        rw_channel.0.send(Arc::new(msg)).await?;
        let handler_list = self.handler_list.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = rw_channel.1.recv().await {
                    let mut res_msg = None;
                    for handler in handler_list.iter() {
                        let res = handler.run(msg.clone(), &mut handler_parameters).await;
                        res_msg = match res {
                            Ok(success) => Some(success),
                            Err(e) => {
                                let err = e.downcast::<HandlerError>();
                                match err {
                                    Ok(err) => match err {
                                        HandlerError::NotMine => None,
                                        HandlerError::Auth { .. } => Some(Msg::err_msg_str(
                                            0,
                                            msg.sender(),
                                            0,
                                            msg.sender_node(),
                                            "auth failed.",
                                        )),
                                        HandlerError::Parse(cause) => Some(Msg::err_msg(
                                            0,
                                            msg.sender(),
                                            0,
                                            msg.sender_node(),
                                            cause,
                                        )),
                                    },
                                    Err(_) => {
                                        error!("unhandled error: {}", err.as_ref().err().unwrap());
                                        None
                                    }
                                }
                            }
                        };
                        match res_msg {
                            None => {
                                continue;
                            }
                            Some(_) => {
                                break;
                            }
                        }
                    }
                    match res_msg {
                        Some(res_msg) => {
                            if res_msg.is_no_op() {
                                continue;
                            }
                            if let Err(_) = rw_channel.0.send(Arc::new(res_msg)).await {
                                error!("send failed.");
                                break;
                            }
                        }
                        None => {
                            let res_msg = Msg::err_msg_str(
                                0,
                                msg.sender(),
                                0,
                                msg.sender_node(),
                                "unknown msg type",
                            );
                            if let Err(_) = rw_channel.0.send(Arc::new(res_msg)).await {
                                error!("send failed.");
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }
}
