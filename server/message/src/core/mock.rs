use crate::cache::{get_redis_ops, TOKEN_KEY};
use crate::config::CONFIG;
use anyhow::anyhow;
use common::net::{MsgIO, Result, ALPN_PRIM};
use common::util::jwt::simple_token;

use quinn::{Connection, Endpoint, RecvStream, SendStream, VarInt};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use common::entity::{Msg, Type};
use tracing::{error, info};

pub(super) struct Client {
    connection: Connection,
    streams: (SendStream, RecvStream),
    endpoint: Endpoint,
}

impl Client {
    #[allow(unused)]
    pub(super) async fn new(address: Option<String>) -> Result<Self> {
        let cert = CONFIG.server.cert.clone();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = quinn::Endpoint::client(
            address
                .unwrap_or("[::1]:0".to_string())
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
        )?;
        endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(client_crypto)));
        let new_connection = endpoint
            .connect(
                "[::1]:11120".to_socket_addrs().unwrap().next().unwrap(),
                "localhost",
            )
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let streams = connection.open_bi().await.unwrap();
        Ok(Self {
            connection,
            streams,
            endpoint,
        })
    }

    #[allow(unused)]
    pub(super) async fn echo(self) -> Result<()> {
        let Self {
            connection,
            streams: (mut send, mut recv),
            endpoint,
        } = self;
        tokio::spawn(async move {
            let mut buffer = [0; 4];
            for _ in 0..11 {
                let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
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
        let mut auth = Msg::auth(115, 0, token);
        MsgIO::write_msg(Arc::new(auth), &mut send).await?;
        for i in 0..10 {
            let mut msg = Msg::text(115, 0, format!("echo: {}", i));
            msg.update_type(Type::Echo);
            MsgIO::write_msg(Arc::new(msg), &mut send).await?;
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
        let _ = tokio::spawn(async move {
            let Self {
                connection,
                streams: (mut send, mut recv),
                endpoint,
            } = client1;
            tokio::spawn(async move {
                let mut buffer = [0; 4];
                loop {
                    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
                    if msg.is_ok() {
                        let msg = msg.unwrap();
                        if msg.typ() == Type::Ack {
                            continue;
                        }
                        info!("client1 get: {}", String::from_utf8_lossy(msg.payload()));
                    } else {
                        error!("client1 read error {}", msg.err().unwrap());
                        break;
                    }
                }
            });
            let _ = get_redis_ops()
                .await
                .set(format!("{}{}", TOKEN_KEY, user_id1), "key")
                .await;
            let token = simple_token("key".to_string(), user_id1);
            let auth = Msg::auth(user_id1, 0, token);
            let _ = MsgIO::write_msg(Arc::new(auth), &mut send).await;
            tokio::time::sleep(Duration::from_millis(1000)).await;
            for i in 0..10 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let mut msg = Msg::text(user_id1, user_id2, format!("echo: {}", i));
                msg.update_type(Type::Echo);
                let _ = MsgIO::write_msg(Arc::new(msg), &mut send).await;
            }
            let _ = send.finish().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            connection.close(VarInt::from(0_u8), "connection done.".as_bytes());
            endpoint.wait_idle().await;
        });
        let _ = tokio::spawn(async move {
            let Self {
                connection,
                streams: (mut send, mut recv),
                endpoint,
            } = client2;
            tokio::spawn(async move {
                let mut buffer = [0; 4];
                loop {
                    let msg = MsgIO::read_msg(&mut buffer, &mut recv).await;
                    if msg.is_ok() {
                        let msg = msg.unwrap();
                        if msg.typ() == Type::Ack {
                            continue;
                        }
                        info!("client2 get: {}", String::from_utf8_lossy(msg.payload()));
                    } else {
                        error!("client2 read error {}", msg.err().unwrap());
                        break;
                    }
                }
            });
            let _ = get_redis_ops()
                .await
                .set(format!("{}{}", TOKEN_KEY, user_id2), "key")
                .await;
            let token = simple_token("key".to_string(), user_id2);
            let auth = Msg::auth(user_id2, 0, token);
            let _ = MsgIO::write_msg(Arc::new(auth), &mut send).await;
            tokio::time::sleep(Duration::from_millis(3000)).await;
            for i in 10..20 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let mut msg = Msg::text(user_id2, user_id1, format!("echo: {}", i));
                msg.update_type(Type::Echo);
                let _ = MsgIO::write_msg(Arc::new(msg), &mut send).await;
            }
            let _ = send.finish().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            connection.close(VarInt::from(0_u8), "connection done.".as_bytes());
            endpoint.wait_idle().await;
        });
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }

    // #[allow(unused)]
    // #[inline]
    // pub(super) async fn read_msg(buffer: &mut LenBuffer, recv: &mut RecvStream) -> Result<Msg> {
    //     let readable = recv.read_exact(&mut buffer[..]).await;
    //     match readable {
    //         Ok(_) => {}
    //         Err(e) => {
    //             return match e {
    //                 ReadExactError::FinishedEarly => {
    //                     info!("stream finished.");
    //                     Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //                         "stream finished.".to_string()
    //                     )))
    //                 }
    //                 ReadExactError::ReadError(e) => {
    //                     warn!("read stream error: {:?}", e);
    //                     Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //                         "read stream error.".to_string()
    //                     )))
    //                 }
    //             }
    //         }
    //     }
    //     let extension_size = Msg::read_u16(&buffer[0..2]);
    //     let payload_size = Msg::read_u16(&buffer[2..4]);
    //     if (payload_size + extension_size) as usize > BODY_SIZE {
    //         return Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //             "message size too large.".to_string()
    //         )));
    //     }
    //     let mut msg = Msg::pre_alloc(payload_size, extension_size);
    //     msg.update_payload_length(payload_size);
    //     msg.update_extension_length(extension_size);
    //     let size = recv
    //         .read_exact(
    //             &mut (msg.as_mut_slice()
    //                 [4..(HEAD_LEN + extension_size as usize + payload_size as usize)]),
    //         )
    //         .await;
    //     match size {
    //         Ok(_) => {}
    //         Err(e) => {
    //             return match e {
    //                 ReadExactError::FinishedEarly => {
    //                     info!("stream finished.");
    //                     Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //                         "stream finished.".to_string()
    //                     )))
    //                 }
    //                 ReadExactError::ReadError(e) => {
    //                     warn!("read stream error: {:?}", e);
    //                     Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //                         "read stream error.".to_string()
    //                     )))
    //                 }
    //             }
    //         }
    //     }
    //     Ok(msg)
    // }
    //
    // #[allow(unused)]
    // #[inline]
    // pub(super) async fn write_msg(msg: &mut Msg, send: &mut SendStream) -> Result<()> {
    //     let res = send.write_all(msg.as_bytes().as_slice()).await;
    //     if let Err(e) = res {
    //         send.finish().await;
    //         warn!("write stream error: {:?}", e);
    //         return Err(anyhow!(crate::error::CrashError::ShouldCrash(
    //             "write stream error.".to_string()
    //         )));
    //     }
    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test() {}
}
