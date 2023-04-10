use ahash::AHashMap;
use lib::{
    entity::{ServerInfo, ServerStatus, ServerType},
    net::{
        client::{ClientConfigBuilder, ClientTimeout},
        server::{Handler, HandlerList},
    },
    Result,
};

use tracing::error;

use crate::{
    config::CONFIG,
    get_io_task_sender,
    service::handler::{
        business::{AddFriend, JoinGroup, LeaveGroup, RemoveFriend, SystemMessage},
        control_text::ControlText,
    },
    util::my_id,
};

use super::{
    handler::internal::{NodeRegister, NodeUnregister},
    SCHEDULER_SENDER,
};

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let addresses = &CONFIG.scheduler.addresses;
        let index = my_id() as usize % addresses.len();
        let scheduler_address = addresses[index].clone();
        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(scheduler_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();
        let mut client = ClientTimeout::new(client_config, std::time::Duration::from_millis(3000));
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
        let (sender, receiver, timeout) = client.io_channel_server_info(&server_info, 0).await?;
        unsafe {
            SCHEDULER_SENDER = Some(lib::net::MsgSender::Client(sender.clone()));
        }

        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(NodeRegister {}));
        handler_list.push(Box::new(NodeUnregister {}));
        handler_list.push(Box::new(ControlText {}));
        handler_list.push(Box::new(JoinGroup {}));
        handler_list.push(Box::new(LeaveGroup {}));
        handler_list.push(Box::new(AddFriend {}));
        handler_list.push(Box::new(RemoveFriend {}));
        handler_list.push(Box::new(SystemMessage {}));
        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let mut inner_states = AHashMap::new();

        if let Err(e) = super::handler::handler_func(
            sender,
            receiver,
            timeout,
            io_task_sender,
            &handler_list,
            &mut inner_states,
        )
        .await
        {
            error!("handler_func error: {}", e);
        }
        Ok(())
    }
}
