use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use common::entity::{Msg, Type};
use common::net::client::ClientConfigBuilder;
use common::util::jwt::simple_token;
use common::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

pub(super) async fn echo(user_id1: u64, user_id2: u64) -> Result<()> {
    let mut client_config = ClientConfigBuilder::default();
    client_config
        .with_remote_address(CONFIG.server.cluster_address)
        .with_domain("localhost".to_string())
        .with_cert(CONFIG.server.cert.clone())
        .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
        .with_max_bi_streams(CONFIG.transport.max_bi_streams)
        .with_max_uni_streams(CONFIG.transport.max_uni_streams);
    let config = client_config.build().unwrap();
    let mut client1 = common::net::client::Client::new(config.clone());
    client1.run().await?;
    let mut client2 = common::net::client::Client::new(config);
    client2.run().await?;
    let _ = get_redis_ops()
        .await
        .set(format!("{}{}", TOKEN_KEY, user_id1), "key")
        .await;
    let _ = get_redis_ops()
        .await
        .set(format!("{}{}", TOKEN_KEY, user_id2), "key")
        .await;
    let streams1 = client1
        .rw_streams(115, 0, simple_token(b"key", 115))
        .await
        .unwrap();
    let streams2 = client2
        .rw_streams(916, 0, simple_token(b"key", 916))
        .await
        .unwrap();
    client1.new_net_streams().await?;
    client2.new_net_streams().await?;
    tokio::spawn(async move {
        let (send, mut recv) = streams1;
        tokio::spawn(async move {
            loop {
                let msg = recv.recv().await;
                if let Some(msg) = msg {
                    if msg.typ() == Type::Ack {
                        continue;
                    }
                    info!("client1: {}", msg);
                }
            }
        });
        tokio::time::sleep(Duration::from_millis(500)).await;
        for i in 0..10 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let msg = Msg::text(user_id1, user_id2, 0, 0, format!("echo: {}", i));
            let _ = send.send(Arc::new(msg)).await;
        }
    });
    tokio::spawn(async move {
        let (send, mut recv) = streams2;
        tokio::spawn(async move {
            loop {
                let msg = recv.recv().await;
                if let Some(msg) = msg {
                    if msg.typ() == Type::Ack {
                        continue;
                    }
                    info!("client2: {}", msg);
                }
            }
        });
        for i in 10..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let msg = Msg::text(user_id2, user_id1, 0, 0, format!("echo: {}", i));
            let _ = send.send(Arc::new(msg)).await;
        }
    });
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {}
}
