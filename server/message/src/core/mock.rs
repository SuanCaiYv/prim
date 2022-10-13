use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use crate::core::{Buffer, Result};
use crate::core::{ALPN_PRIM, BODY_BUF_LENGTH};
use crate::entity::{msg, HEAD_LEN};
use crate::util::jwt::simple_token;
use anyhow::anyhow;

use quinn::{Endpoint, RecvStream, SendStream};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

pub(super) struct Client {
    connection: (SendStream, RecvStream),
    endpoint: Endpoint,
}

impl Client {
    #[allow(unused)]
    pub(super) async fn new() -> Result<Self> {
        let cert = CONFIG.server.cert.clone();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint =
            quinn::Endpoint::client("[::1]:0".to_socket_addrs().unwrap().next().unwrap())?;
        endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(client_crypto)));
        let new_connection = endpoint
            .connect(
                "[::1]:11120".to_socket_addrs().unwrap().next().unwrap(),
                "localhost",
            )
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection {
            connection: conn, ..
        } = new_connection;
        Ok(Self {
            connection: conn.open_bi().await.unwrap(),
            endpoint,
        })
    }

    #[allow(unused)]
    pub(super) async fn echo(self) -> Result<()> {
        let Self {
            connection: (mut send, mut recv),
            endpoint,
        } = self;
        tokio::spawn(async move {
            let buffer = &mut Buffer {
                head_buf: [0; HEAD_LEN],
                body_buf: Box::new([0; BODY_BUF_LENGTH]),
            };
            for _ in 0..11 {
                let msg = super::server::ConnectionTask::read_msg(buffer, &mut recv).await;
                if msg.is_ok() {
                    info!("get: {}", msg.unwrap());
                }
            }
        });
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, 115), "key")
            .await?;
        let token = simple_token("key".to_string(), 115);
        let auth = msg::Msg::auth(115, 0, token);
        super::server::ConnectionTask::write_msg(&auth, &mut send).await?;
        for i in 0..10 {
            let mut msg = msg::Msg::text(115, 0, format!("echo: {}", i));
            msg.head.typ = msg::Type::Echo;
            super::server::ConnectionTask::write_msg(&msg, &mut send).await?;
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
        send.finish().await?;
        endpoint.wait_idle().await;
        Ok(())
    }

    pub(super) async fn echo_you_and_me(
        client1: Client,
        client2: Client,
        user_id1: u64,
        user_id2: u64,
    ) -> Result<()> {
        tokio::spawn(async move {
            let Self {
                connection: (mut send, mut recv),
                endpoint,
            } = client1;
            tokio::spawn(async move {
                let buffer = &mut Buffer {
                    head_buf: [0; HEAD_LEN],
                    body_buf: Box::new([0; BODY_BUF_LENGTH]),
                };
                loop {
                    let msg = super::server::ConnectionTask::read_msg(buffer, &mut recv).await;
                    if msg.is_ok() {
                        let msg = msg.unwrap();
                        if msg.head.typ == msg::Type::Ack {
                            continue;
                        }
                        info!("client1 get: {}", msg);
                    } else {
                        break;
                    }
                }
            });
            let _ = get_redis_ops()
                .await
                .set(format!("{}{}", TOKEN_KEY, user_id1), "key")
                .await;
            let token = simple_token("key".to_string(), user_id1);
            let auth = msg::Msg::auth(user_id1, 0, token);
            let _ = super::server::ConnectionTask::write_msg(&auth, &mut send).await;
            for i in 10..20 {
                let mut msg = msg::Msg::text(user_id1, user_id2, format!("echo: {}", i));
                msg.head.typ = msg::Type::Echo;
                let _ = super::server::ConnectionTask::write_msg(&msg, &mut send).await;
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let _ = send.finish().await;
            endpoint.wait_idle().await;
        });
        let Self {
            connection: (mut send, mut recv),
            endpoint,
        } = client2;
        tokio::spawn(async move {
            let buffer = &mut Buffer {
                head_buf: [0; HEAD_LEN],
                body_buf: Box::new([0; BODY_BUF_LENGTH]),
            };
            loop {
                let msg = super::server::ConnectionTask::read_msg(buffer, &mut recv).await;
                if msg.is_ok() {
                    let msg = msg.unwrap();
                    if msg.head.typ == msg::Type::Ack {
                        continue;
                    }
                    info!("client2 get: {}", msg);
                } else {
                    break;
                }
            }
        });
        get_redis_ops()
            .await
            .set(format!("{}{}", TOKEN_KEY, user_id2), "key")
            .await?;
        let token = simple_token("key".to_string(), user_id2);
        let auth = msg::Msg::auth(user_id2, user_id1, token);
        super::server::ConnectionTask::write_msg(&auth, &mut send).await?;
        for i in 0..10 {
            let mut msg = msg::Msg::text(user_id2, user_id1, format!("echo: {}", i));
            msg.head.typ = msg::Type::Echo;
            super::server::ConnectionTask::write_msg(&msg, &mut send).await?;
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
        send.finish().await?;
        endpoint.wait_idle().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {}
}
