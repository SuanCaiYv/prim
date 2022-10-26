use crate::cache::{get_redis_ops, TOKEN_KEY};
use common::entity::Msg;
use common::net::client::{Client, ClientConfigBuilder};
use common::util::jwt::simple_token;
use common::util::salt;
use common::Result;
use local_ip_address::list_afinet_netifas;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::config::CONFIG;
use crate::onstart::my_id;

pub(crate) async fn registry_self() -> Result<()> {
    let list = list_afinet_netifas().unwrap();
    let ip = list
        .iter()
        .filter(|(name, addr)| {
            if name == "en0" {
                if let IpAddr::V6(_) = addr {
                    return true;
                }
            }
            false
        })
        .map(|x| x.1)
        .collect::<Vec<IpAddr>>();
    let my_ip = ip[0].to_string();
    let my_address = format!("[{}]:{}", my_ip, CONFIG.server.address.port());
    let my_id = my_id().await;
    let mut client_config = ClientConfigBuilder::default();
    let addresses = &CONFIG.balancer.addresses;
    let index = my_id as usize % addresses.len();
    let balancer_address = addresses[index].clone();
    client_config
        .with_address(balancer_address)
        .with_domain(CONFIG.balancer.domain.clone())
        .with_cert(CONFIG.balancer.cert.clone())
        .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams);
    let config = client_config.build().unwrap();
    let mut client = Client::new(config.clone(), my_id);
    client.run().await?;
    let token_key = salt();
    let token = simple_token(token_key.as_bytes(), my_id);
    get_redis_ops()
        .await
        .set(format!("{}{}", TOKEN_KEY, my_id), token_key)
        .await?;
    let stream = client.rw_streams(my_id, token).await?;
    stream
        .0
        .send(Arc::new(Msg::text(my_id, 0, my_address)))
        .await?;
    tokio::time::sleep(Duration::MAX).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use local_ip_address::list_afinet_netifas;
    use std::net::IpAddr;

    #[test]
    fn test() {
        let list = list_afinet_netifas().unwrap();
        let ip = list
            .iter()
            .filter(|(name, addr)| {
                if name == "en0" {
                    if let IpAddr::V6(_) = addr {
                        return true;
                    }
                }
                false
            })
            .map(|x| x.1)
            .collect::<Vec<IpAddr>>();
        println!("{:?}", ip[0])
    }
}
