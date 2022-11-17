use ahash::AHashMap;
use anyhow::anyhow;
use std::sync::Arc;
use tracing::error;

use crate::config::CONFIG;
use crate::util::my_id;
use common::entity::{Msg, ServerInfo, ServerStatus, ServerType, Type};
use common::error::HandlerError;
use common::net::client::{Client, ClientConfigBuilder};
use common::net::server::{GenericParameterMap, HandlerList, HandlerParameters};
use common::net::InnerSender;
use common::Result;

pub(crate) struct ClientToBalancer {
    pub(crate) io_handler_sender: InnerSender,
    pub(crate) handler_list: HandlerList,
}

impl ClientToBalancer {
    pub(crate) async fn run(&self) -> Result<()> {
        let my_address = CONFIG.server.service_address;
        let m_id = my_id();
        let addresses = &CONFIG.scheduler.addresses;
        let index = my_id as usize % addresses.len();
        let balancer_address = addresses[index].clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(balancer_address)
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let config = client_config.build().unwrap();
        let mut client = Client::new(config.clone());
        client.run().await?;
        let server_info = ServerInfo {
            id: m_id,
            address: my_address,
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::BalancerNode,
            load: None,
        };
        let payload = server_info.to_bytes();
        let mut msg = Msg::raw_payload(&payload);
        msg.set_type(Type::Auth);
        msg.set_sender(server_info.id as u64);
        msg.set_sender_node(server_info.id);
        let mut io_channel = client.rw_streams_no_token().await?;
        io_channel.0.send(Arc::new(msg)).await?;
        let mut handler_parameters = HandlerParameters {
            io_handler_sender: self.io_handler_sender.clone(),
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        loop {
            if let Some(msg) = io_channel.1.recv().await {
                let mut res_msg = None;
                for handler in self.handler_list.iter() {
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
                        if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
                            error!("send failed.");
                            return Err(anyhow!("send failed."));
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
                        if let Err(_) = io_channel.0.send(Arc::new(res_msg)).await {
                            error!("send failed.");
                            return Err(anyhow!("send failed."));
                        }
                    }
                }
            }
        }
    }
}
