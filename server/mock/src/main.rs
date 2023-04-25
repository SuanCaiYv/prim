use std::{sync::Arc, time::Duration};

use ahash::{AHashMap, AHashSet};

use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::StreamExt;
use lib::{
    entity::ReqwestMsg,
    joy,
    net::{
        client::{ClientConfigBuilder, ClientReqwest},
        server::{
            GenericParameterMap, HandlerParameters, InnerStates, NewReqwestConnectionHandler,
            NewReqwestConnectionHandlerGenerator, ReqwestHandler, ReqwestHandlerList,
            ServerConfigBuilder, ServerReqwest,
        },
        ReqwestMsgIOUtil, ReqwestMsgIOWrapper,
    },
    Result,
};

use quinn::Endpoint;
use tokio::select;
use tracing::{error, info};

use crate::config::CONFIG;

mod cache;
mod config;
mod util;

struct Echo {}

#[async_trait]
impl ReqwestHandler for Echo {
    async fn run(
        &self,
        msg: &ReqwestMsg,
        _parameters: &mut HandlerParameters,
        // this one contains some states corresponding to the quic stream.
        _inner_states: &mut InnerStates,
    ) -> Result<ReqwestMsg> {
        // let req_id = msg.req_id();
        // let resource_id = msg.resource_id();
        // let mut number = String::from_utf8_lossy(msg.payload())
        //     .to_string()
        //     .parse::<u64>();
        // if number.is_err() {
        //     error!("failed num: {}", String::from_utf8_lossy(msg.payload()));
        //     number = Ok(111);
        // }
        // let number = number.unwrap();
        // let resp = format!(
        //     "hello client, you are:{:06} have required for {:06} with {:06}.",
        //     req_id, resource_id, number
        // );
        // let mut resp_msg = ReqwestMsg::with_resource_id_payload(resource_id, resp.as_bytes());
        // resp_msg.set_req_id(req_id);
        // Ok(resp_msg)
        Ok(msg.clone())
    }
}

struct ReqwestMessageHandler {
    handler_list: ReqwestHandlerList,
}

#[async_trait]
impl NewReqwestConnectionHandler for ReqwestMessageHandler {
    async fn handle(&mut self, mut io_operators: ReqwestMsgIOWrapper) -> Result<()> {
        let (sender, mut receiver) = io_operators.channels();
        let mut parameters = HandlerParameters {
            generic_parameters: GenericParameterMap(AHashMap::new()),
        };
        let mut inner_states = InnerStates::new();
        loop {
            let msg = receiver.recv().await;
            match msg {
                Some(msg) => {
                    match self.handler_list[0]
                        .run(&msg, &mut parameters, &mut inner_states)
                        .await
                    {
                        Ok(resp) => {
                            if let Err(e) = sender.send(resp).await {
                                error!("{}", e);
                            }
                        }
                        Err(e) => {
                            error!("handler error: {}", e);
                        }
                    }
                }
                None => {
                    break;
                }
            };
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_line_number(true)
                .with_level(true)
                .with_target(true),
        )
        .with_max_level(CONFIG.log_level)
        .try_init()
        .unwrap();
    println!("{}", joy::banner());
    let mut server_config_builder = ServerConfigBuilder::default();
    server_config_builder.with_address("0.0.0.0:8190".parse().unwrap());
    server_config_builder.with_connection_idle_timeout(3000);
    server_config_builder.with_max_bi_streams(8);
    server_config_builder.with_max_connections(100);
    server_config_builder.with_cert(CONFIG.server.cert.clone());
    server_config_builder.with_key(CONFIG.server.key.clone());
    let mut client_config_builder = ClientConfigBuilder::default();
    client_config_builder.with_remote_address("127.0.0.1:8190".parse().unwrap());
    client_config_builder.with_domain("localhost".to_string());
    client_config_builder.with_ipv4_type(true);
    client_config_builder.with_max_bi_streams(8);
    client_config_builder.with_keep_alive_interval(Duration::from_millis(2000));
    client_config_builder.with_cert(CONFIG.scheduler.cert.clone());
    let server_config = server_config_builder.build().unwrap();
    let client_config = client_config_builder.build().unwrap();
    let mut handler_list: Vec<Box<dyn ReqwestHandler>> = Vec::new();
    handler_list.push(Box::new(Echo {}));
    let handler_list = ReqwestHandlerList::new(handler_list);
    let generator: NewReqwestConnectionHandlerGenerator = Box::new(move || {
        Box::new(ReqwestMessageHandler {
            handler_list: handler_list.clone(),
        })
    });
    let mut server = ServerReqwest::new(server_config);
    let mut client = ClientReqwest::new(client_config);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        _ = client.build().await;
        let client = Arc::new(client);
        for i in 0..20000 {
            let client = client.clone();
            tokio::spawn(async move {
                let resource_id = fastrand::u16(..);
                let req = ReqwestMsg::with_resource_id_payload(
                    resource_id,
                    format!("{:06}", i).as_bytes(),
                );
                match client.call(req).await {
                    Ok(resp) => {
                        // info!("resp: {}", String::from_utf8_lossy(resp.payload()));
                    }
                    Err(e) => {
                        error!("call error: {}", e);
                    }
                }
            });
        }
        tokio::time::sleep(Duration::from_millis(8000)).await;
    });
    // tokio::spawn(async move {
    //     test().await;
    // });
    if let Err(e) = server.run(generator).await {
        error!("reqwest server error: {}", e);
    }
    // tokio::time::sleep(Duration::from_millis(4000)).await;
    Ok(())
}

async fn test() -> Result<()> {
    // tokio::spawn(async move {
    //     let mut server_crypto = rustls::ServerConfig::builder()
    //         .with_safe_defaults()
    //         .with_no_client_auth()
    //         .with_single_cert(vec![CONFIG.server.cert.clone()], CONFIG.server.key.clone())
    //         .unwrap();
    //     server_crypto.alpn_protocols = vec![vec![b'p', b'r', b'i', b'm']];
    //     let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
    //     quinn_server_config.concurrent_connections(10000);
    //     quinn_server_config.use_retry(true);
    //     Arc::get_mut(&mut quinn_server_config.transport)
    //         .unwrap()
    //         .max_concurrent_bidi_streams(quinn::VarInt::from_u64(8).unwrap())
    //         .max_idle_timeout(Some(quinn::IdleTimeout::from(
    //             quinn::VarInt::from_u64(3000).unwrap(),
    //         )));
    //     let (endpoint, mut incoming) =
    //         quinn::Endpoint::server(quinn_server_config, "0.0.0.0:8190".parse().unwrap()).unwrap();
    //     while let Some(conn) = incoming.next().await {
    //         let mut conn = conn.await.unwrap();
    //         info!(
    //             "new connection: {}",
    //             conn.connection.remote_address().to_string()
    //         );
    //         tokio::spawn(async move {
    //             loop {
    //                 if let Some(streams) = conn.bi_streams.next().await {
    //                     let io_streams = match streams {
    //                         Ok(ok) => ok,
    //                         _ => {
    //                             return Err(anyhow!("quinn stream error"));
    //                         }
    //                     };
    //                     info!("new streams");
    //                     let (mut send_stream, mut recv_stream) = io_streams;
    //                     tokio::spawn(async move {
    //                         // let mut buf = [0u8; 64];
    //                         let mut bytes = 0;
    //                         loop {
    //                             match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None,4 ).await {
    //                                 Ok(msg) => {
    //                                     bytes += msg.as_slice().len();
    //                                     // info!("recv: {}", String::from_utf8_lossy(&buf));
    //                                     ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await;
    //                                 }
    //                                 Err(e) => {
    //                                     error!("recv error: {}", e);
    //                                     break;
    //                                 }
    //                             }
    //                         }
    //                         // info!("recv bytes: {}", bytes);
    //                     });
    //                 } else {
    //                     break;
    //                 }
    //             }
    //             conn.connection
    //                 .close(0u32.into(), b"it's time to say goodbye.");
    //             Ok(())
    //         });
    //     }
    //     endpoint.wait_idle().await;
    // });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&CONFIG.scheduler.cert)?;
    let mut client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![vec![b'p', b'r', b'i', b'm']];
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
    let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
    Arc::get_mut(&mut client_config.transport)
        .unwrap()
        .max_concurrent_bidi_streams(quinn::VarInt::from_u64(8).unwrap())
        .keep_alive_interval(Some(Duration::from_millis(2000)));
    endpoint.set_default_client_config(client_config);
    let new_connection = endpoint
        .connect("127.0.0.1:8190".parse().unwrap(), "localhost")
        .unwrap()
        .await
        .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
    let quinn::NewConnection { connection, .. } = new_connection;
    let mut sender_list = vec![];
    for _ in 0..8 {
        let (mut send_stream, mut recv_stream) = match connection.open_bi().await {
            Ok(v) => v,
            Err(e) => {
                error!("open streams error: {}", e.to_string());
                continue;
            }
        };
        let (send_sender, mut send_receiver) = tokio::sync::mpsc::channel::<ReqwestMsg>(64);
        tokio::spawn(async move {
            loop {
                match send_receiver.recv().await {
                    Some(msg) => {
                        if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await {
                            error!("send msg error: {:?}", e);
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        });
        tokio::spawn(async move {
            let mut set = AHashSet::new();
            loop {
                match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None, 3).await {
                    Ok(msg) => {
                        set.insert(msg.req_id());
                    }
                    Err(e) => {
                        error!("recv error: {}", e);
                        break;
                    }
                }
            }
            info!("recv bytes: {}", set.len());
        });
        sender_list.push(send_sender);
    }
    for i in 0..10000 {
        let sender = sender_list.get(i % 8).unwrap().clone();
        tokio::spawn(async move {
            let mut msg = ReqwestMsg::with_resource_id_payload(1, format!("{:06}", 2).as_bytes());
            msg.set_req_id(i as u64);
            if let Err(e) = sender.send(msg).await {
                error!("send msg error: {:?}", e);
            }
        });
    }
    tokio::time::sleep(Duration::from_millis(3000)).await;
    Ok(())
}
