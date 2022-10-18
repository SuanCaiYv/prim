use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;

use common::net::client::ClientConfigBuilder;
use common::net::Result;
use common::util::jwt::simple_token;

use std::sync::Arc;
use std::time::Duration;

use common::entity::{Msg, Type};
use tracing::info;

pub(super) async fn echo(user_id1: u64, user_id2: u64) -> Result<()> {
    let mut client_config = ClientConfigBuilder::default();
    client_config
        .with_address(CONFIG.server.address)
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
    {
        client1.new_streams().await?;
        client2.new_streams().await?;
    }
    {
        client1.new_streams().await?;
        client2.new_streams().await?;
    }
    let streams1 = client1.streams().unwrap();
    let streams2 = client2.streams().unwrap();
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
        let _ = get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, user_id1), "key")
            .await;
        let token = simple_token("key".to_string(), user_id1);
        let auth = Msg::auth(user_id1, 0, token);
        let _ = send.send(Arc::new(auth)).await;
        tokio::time::sleep(Duration::from_millis(3000)).await;
        for i in 0..10 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut msg = Msg::text(user_id1, user_id2, format!("echo: {}", i));
            msg.update_type(Type::Echo);
            let _ = send.send(Arc::new(msg)).await;
        }
        let _ = client1.wait_for_closed().await;
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
        let _ = get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, user_id2), "key")
            .await;
        let token = simple_token("key".to_string(), user_id2);
        let auth = Msg::auth(user_id2, 0, token);
        let _ = send.send(Arc::new(auth)).await;
        tokio::time::sleep(Duration::from_millis(1000)).await;
        for i in 10..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut msg = Msg::text(user_id2, user_id1, format!("echo: {}", i));
            msg.update_type(Type::Echo);
            let _ = send.send(Arc::new(msg)).await;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = client2.wait_for_closed().await;
    });
    Ok(())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {}
}
