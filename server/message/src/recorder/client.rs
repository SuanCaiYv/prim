use std::sync::Arc;

use ahash::AHashMap;
use anyhow::anyhow;
use lib::{
    entity::{Msg, ServerInfo, ServerStatus, ServerType, Type},
    net::client::{ClientConfigBuilder, ClientTimeout},
    Result,
};
use tracing::{debug, error};

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
            .with_domain(CONFIG.recorder.domain.clone())
            .with_cert(CONFIG.recorder.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams)
            .with_max_sender_side_channel_size(CONFIG.performance.max_sender_side_channel_size)
            .with_max_receiver_side_channel_size(CONFIG.performance.max_receiver_side_channel_size);
        let config = client_config.build().unwrap();
        let mut client = ClientTimeout::new(config, std::time::Duration::from_millis(3000));
        client.run().await?;
        let (io_sender, mut io_receiver, mut timeout_receiver) = client.io_channel().await?;
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
        let recorder_id = res_server_info.id;
        let sender = io_sender.clone();
        unsafe { RECORDER_SENDER = Some(sender) }
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
                                    if let Err(e) = io_sender.send(failed_msg).await {
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
