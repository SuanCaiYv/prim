use std::time::Duration;

use lib::{
    entity::{ServerInfo, ServerStatus, ServerType},
    net::{
        client::{ClientConfigBuilder, ClientTimeout},
        server::{Handler, HandlerList, InnerStates},
    },
    Result,
};
use tracing::{debug, error};

use crate::{
    cluster::{
        handler::{logic, message},
        MsgSender,
    },
    config::CONFIG,
    util::my_id,
};

use super::get_cluster_connection_set;
pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let cluster_set = get_cluster_connection_set();
        let mut addr_vec = CONFIG.cluster.addresses.clone();
        let my_addr = CONFIG.server.cluster_address;
        addr_vec.sort();
        let num = (addr_vec.len() - 1) / 2;
        let mut index = 0;
        for addr in addr_vec.iter() {
            index += 1;
            if *addr == my_addr {
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        for _ in 0..num {
            let i = index % addr_vec.len();
            index += 1;
            let addr = &addr_vec[i];
            if cluster_set.contains(&addr) {
                continue;
            }
            let ipv4 = CONFIG.server.cluster_address.is_ipv4();
            let mut client_config = ClientConfigBuilder::default();
            client_config
                .with_remote_address(addr.to_owned())
                .with_ipv4_type(ipv4)
                .with_domain(CONFIG.server.domain.clone())
                .with_cert(CONFIG.cluster.cert.clone())
                .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
                .with_max_bi_streams(CONFIG.transport.max_bi_streams);
            let client_config = client_config.build().unwrap();

            let mut client = ClientTimeout::new(client_config, Duration::from_millis(3000));
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
                typ: ServerType::SchedulerCluster,
                load: None,
            };
            let (sender, receiver, timeout) =
                client.io_channel_server_info(&server_info, 0).await?;
            debug!("cluster client {} connected", addr);

            let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
            handler_list.push(Box::new(logic::ClientAuth {}));
            handler_list.push(Box::new(message::NodeRegister {}));
            handler_list.push(Box::new(message::NodeUnregister {}));
            let handler_list = HandlerList::new(handler_list);
            let mut inner_states = InnerStates::new();
            tokio::spawn(async move {
                // try to extend the lifetime of client to avoid being dropped.
                let _client = client;
                if let Err(e) = super::handler::handler_func(
                    MsgSender::Client(sender),
                    receiver,
                    timeout,
                    &handler_list,
                    &mut inner_states,
                )
                .await
                {
                    error!("handler_func error: {}", e);
                }
            });
        }
        Ok(())
    }
}
