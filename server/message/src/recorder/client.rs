use ahash::AHashMap;

use lib::{
    entity::{ServerInfo, ServerStatus, ServerType},
    net::client::{ClientConfigBuilder, ClientTimeout},
    Result,
};
use tracing::error;

use crate::{config::CONFIG, util::my_id};

use super::RECORDER_SENDER;

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        // todo unix domain socket optimization and dynamically get recorder address
        let recorder_address = CONFIG.recorder.address.clone();
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_remote_address(recorder_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.recorder.domain.clone())
            .with_cert(CONFIG.recorder.cert.clone())
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
        let (sender, _receiver, mut timeout_receiver, auth_resp) =
            client.io_channel_server_info(&server_info, 0).await?;
        let res_server_info = ServerInfo::from(auth_resp.payload());
        let recorder_id = res_server_info.id;
        unsafe { RECORDER_SENDER = Some(sender.clone()) }
        tokio::spawn(async move {
            let _client = client;
            let mut retry_count = AHashMap::new();
            loop {
                let failed_msg = timeout_receiver.recv().await;
                match failed_msg {
                    Some(failed_msg) => {
                        let key = failed_msg.timestamp() % 4000;
                        match retry_count.get(&key) {
                            Some(count) => {
                                if *count == 0 {
                                    error!(
                                        "retry too many times, peer may busy or dead. msg: {}",
                                        failed_msg
                                    );
                                } else {
                                    retry_count.insert(key, *count - 1);
                                    if let Err(e) = sender.send(failed_msg).await {
                                        error!("retry failed send msg. error: {}", e);
                                        break;
                                    }
                                }
                            }
                            None => {
                                retry_count.insert(key, 4);
                            }
                        }
                    }
                    None => {
                        error!("recorder[{}] crashed.", recorder_id);
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}
