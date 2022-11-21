use std::sync::Arc;

use anyhow::anyhow;
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    net::client::{ClientConfigBuilder, ClientTimeout},
    Result,
};
use tracing::{debug, error};

use crate::{config::CONFIG, util::my_id};

use super::SCHEDULER_SENDER;

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let addresses = &CONFIG.scheduler.addresses;
        let index = my_id() as usize % addresses.len();
        let scheduler_address = addresses[index].clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(scheduler_address)
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let config = client_config.build().unwrap();
        let mut client = ClientTimeout::new(config, std::time::Duration::from_millis(3000));
        client.run().await?;
        let (io_sender, mut io_receiver, timeout_receiver) = client.io_channel().await?;
        let sender = io_sender.clone();
        unsafe {
            SCHEDULER_SENDER = Some(sender);
        }
        let server_info = ServerInfo {
            id: my_id(),
            address: CONFIG.server.cluster_address,
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SchedulerClient,
            load: None,
        };
        let mut auth = Msg::raw_payload(&server_info.to_bytes());
        auth.set_type(Type::Auth);
        auth.set_sender(server_info.id as u64);
        io_sender.send(Arc::new(auth)).await?;
        let res_server_info;
        match io_receiver.recv().await {
            Some(res_msg) => {
                if res_msg.typ() != Type::Auth {
                    error!("auth failed");
                    return Err(anyhow!("auth failed"));
                }
                res_server_info = ServerInfo::from(res_msg.payload());
                debug!("scheduler node: {}", res_server_info);
            }
            None => {
                error!("cluster client io_receiver recv None");
                return Err(anyhow!("cluster client io_receiver closed"));
            }
        }
        // register self to scheduler
        let server_info = ServerInfo {
            id: my_id(),
            address: CONFIG.server.cluster_address,
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MessageCluster,
            load: None,
        };
        let mut register_msg = Msg::raw_payload(&server_info.to_bytes());
        register_msg.set_type(Type::NodeRegister);
        register_msg.set_sender(server_info.id as u64);
        io_sender.send(Arc::new(register_msg)).await?;
        if let Err(e) = super::handler::handler_func(
            (io_sender, io_receiver),
            timeout_receiver,
            &res_server_info,
        )
        .await
        {
            error!("handler_func error: {}", e);
        }
        Ok(())
    }
}
