use std::sync::Arc;

use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    net::client::{ClientConfigBuilder, ClientTimeout},
    Result,
};
use tracing::error;

use crate::{config::CONFIG, service::handler::IOTaskSender, util::my_id};

use super::SCHEDULER_SENDER;

pub(super) struct Client {}

impl Client {
    pub(super) async fn run(io_task_sender: IOTaskSender) -> Result<()> {
        let addresses = &CONFIG.scheduler.addresses;
        let index = my_id() as usize % addresses.len();
        let scheduler_address = addresses[index].clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(scheduler_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let config = client_config.build().unwrap();
        let mut client = ClientTimeout::new(config, std::time::Duration::from_millis(3000));
        client.run().await?;
        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let mut cluster_address = CONFIG.server.cluster_address;
        cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SchedulerClient,
            load: None,
        };
        let (sender, receiver, timeout, auth_resp) =
            client.io_channel_server_info(&server_info, 0).await?;
        unsafe {
            SCHEDULER_SENDER = Some(sender.clone());
        }
        let res_server_info = ServerInfo::from(auth_resp.payload());
        // register self to scheduler
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::MessageCluster,
            load: None,
        };
        let mut register_msg = Msg::raw_payload(&server_info.to_bytes());
        register_msg.set_type(Type::MessageNodeRegister);
        register_msg.set_sender(server_info.id as u64);
        sender.send(Arc::new(register_msg)).await?;
        if let Err(e) = super::handler::handler_func(
            sender,
            receiver,
            io_task_sender,
            timeout,
            &res_server_info,
        )
        .await
        {
            error!("handler_func error: {}", e);
        }
        Ok(())
    }
}
