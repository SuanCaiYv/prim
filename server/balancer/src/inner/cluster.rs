use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use common::net::client::{Client, ClientConfigBuilder};
use common::net::{OuterReceiver, OuterSender};
use common::util::jwt::simple_token;
use common::Result;
use std::net::SocketAddr;
use tracing::{error, warn};

pub(crate) struct BalancerCLusterClient;

impl BalancerCLusterClient {
    pub(crate) async fn run(
        cluster_client_list: &mut Vec<(OuterSender, OuterReceiver, Client)>,
    ) -> Result<()> {
        let addresses = &CONFIG.balancer.addresses;
        let my_address = &CONFIG.server.address;
        for address in addresses.iter() {
            if address == my_address {
                continue;
            }
            let mut res = Self::connect(address.to_owned()).await;
            for _ in 0..5 {
                if res.is_err() {
                    warn!("connect to balancer {} failed", address);
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
                res = Self::connect(address.to_owned()).await;
            }
            if res.is_err() {
                error!("connect to balancer {} failed", address);
                break;
            }
            cluster_client_list.push(res.unwrap());
        }
        Ok(())
    }

    async fn connect(address: SocketAddr) -> Result<(OuterSender, OuterReceiver, Client)> {
        let mut client_config = ClientConfigBuilder::default();
        client_config
            .with_address(address)
            .with_domain(CONFIG.balancer.domain.clone())
            .with_cert(CONFIG.balancer.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams)
            .with_max_uni_streams(CONFIG.transport.max_uni_streams);
        let config = client_config.build().unwrap();
        let mut client = Client::new(config.clone(), 0);
        client.run().await?;
        let token_key = b"balancer_v1gu829edfc8uvygvsbwnqk";
        let token = simple_token(token_key, 0);
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, 0), token_key)
            .await?;
        let stream = client.rw_streams(0, token).await?;
        Ok((stream.0, stream.1, client))
    }
}
