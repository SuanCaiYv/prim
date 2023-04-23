use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use ahash::AHashMap;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures_util::StreamExt;
use lib::{
    entity::ReqwestMsg,
    joy,
    net::{
        server::{
            GenericParameterMap, HandlerParameters, InnerStates, NewReqwestConnectionHandler,
            ReqwestHandler, ReqwestHandlerList, ServerReqwest, NewReqwestConnectionHandlerGenerator, ServerConfigBuilder,
        },
        ReqwestMsgIOUtil, ReqwestMsgIOWrapper, client::{ClientReqwest, ClientConfigBuilder},
    },
    Result,
};
use quinn::{Endpoint, NewConnection};
use tokio::{select, sync::mpsc};

use tracing::{debug, error, info, warn};

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
                    if let Err(_) = sender.send(msg).await {
                        break;
                    }
                },
                None => {}
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
    server_config_builder.with_cert(rustls::Certificate(
        fs::read(PathBuf::from(
            "/Users/joker/RustProjects/prim/server/cert/localhost-server.crt.der",
        ))
        .context("read cert file failed.")
        .unwrap(),
    ));
    server_config_builder.with_key(rustls::PrivateKey(
        fs::read(PathBuf::from(
            "/Users/joker/RustProjects/prim/server/cert/localhost-server.key.der",
        ))
        .context("read key file failed.")
        .unwrap(),
    ));
    let mut client_config_builder = ClientConfigBuilder::default();
    client_config_builder.with_remote_address("127.0.0.1:8190".parse().unwrap());
    client_config_builder.with_domain("localhost".to_string());
    client_config_builder.with_ipv4_type(true);
    client_config_builder.with_max_bi_streams(8);
    client_config_builder.with_keep_alive_interval(Duration::from_millis(2000));
    client_config_builder.with_cert(rustls::Certificate(
        fs::read(PathBuf::from(
            "/Users/joker/RustProjects/prim/server/cert/PrimRootCA.crt.der",
        ))
        .context("read cert file failed.")
        .unwrap(),
    ));
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
        let _sender_list = build().await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    if let Err(e) = server.run(generator).await {
        error!("message server error: {}", e);
    }
    Ok(())
}

async fn serve() -> Result<()> {
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(
            vec![rustls::Certificate(
                fs::read(PathBuf::from(
                    "/Users/joker/RustProjects/prim/server/cert/localhost-server.crt.der",
                ))
                .context("read cert file failed.")
                .unwrap(),
            )],
            rustls::PrivateKey(
                fs::read(PathBuf::from(
                    "/Users/joker/RustProjects/prim/server/cert/localhost-server.key.der",
                ))
                .context("read key file failed.")
                .unwrap(),
            ),
        )?;
    server_crypto.alpn_protocols = vec![vec![b'p', b'r', b'i', b'm']];
    let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
    quinn_server_config.concurrent_connections(1000);
    quinn_server_config.use_retry(true);
    Arc::get_mut(&mut quinn_server_config.transport)
        .unwrap()
        .max_concurrent_bidi_streams(quinn::VarInt::from_u64(8).unwrap())
        .max_idle_timeout(Some(quinn::IdleTimeout::from(
            quinn::VarInt::from_u64(3000).unwrap(),
        )));
    let (endpoint, mut incoming) =
        quinn::Endpoint::server(quinn_server_config, "0.0.0.0:8190".parse().unwrap())?;
    while let Some(conn) = incoming.next().await {
        let conn = conn.await?;
        info!(
            "new connection: {}",
            conn.connection.remote_address().to_string()
        );
        tokio::spawn(async move {
            let _ = handle_new_connection(conn).await;
        });
    }
    endpoint.wait_idle().await;
    Ok(())
}

async fn handle_new_connection(mut conn: NewConnection) -> Result<()> {
    loop {
        if let Some(streams) = conn.bi_streams.next().await {
            let io_streams = match streams {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("the peer close the connection.");
                    Err(anyhow!("the peer close the connection."))
                }
                Err(quinn::ConnectionError::ConnectionClosed { .. }) => {
                    info!("the peer close the connection but by quic.");
                    Err(anyhow!("the peer close the connection but by quic."))
                }
                Err(quinn::ConnectionError::Reset) => {
                    error!("connection reset.");
                    Err(anyhow!("connection reset."))
                }
                Err(quinn::ConnectionError::TransportError { .. }) => {
                    error!("connect by fake specification.");
                    Err(anyhow!("connect by fake specification."))
                }
                Err(quinn::ConnectionError::TimedOut) => {
                    error!("connection idle for too long time.");
                    Err(anyhow!("connection idle for too long time."))
                }
                Err(quinn::ConnectionError::VersionMismatch) => {
                    error!("connect by unsupported protocol version.");
                    Err(anyhow!("connect by unsupported protocol version."))
                }
                Err(quinn::ConnectionError::LocallyClosed) => {
                    error!("local server fatal.");
                    Err(anyhow!("local server fatal."))
                }
                Ok(ok) => Ok(ok),
            };
            if let Ok(io_streams) = io_streams {
                let (mut send_stream, mut recv_stream) = io_streams;
                tokio::spawn(async move {
                    let mut counter = 0;
                    loop {
                        match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, Some(&mut counter)).await {
                            Ok(msg) => {
                                if let Err(e) =
                                    ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream, None).await
                                {
                                    error!("send message error: {}", e);
                                    break;
                                }
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                    warn!("{} recv: {}", recv_stream.id().0, counter);
                });
            } else {
                break;
            }
        } else {
            break;
        }
    }
    debug!("connection closed.");
    conn.connection
        .close(0u32.into(), b"it's time to say goodbye.");
    Ok(())
}

async fn build() -> Result<Vec<mpsc::Sender<ReqwestMsg>>> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&rustls::Certificate(
        fs::read(PathBuf::from(
            "/Users/joker/RustProjects/prim/server/cert/PrimRootCA.crt.der",
        ))
        .context("read cert file failed.")
        .unwrap(),
    ))?;
    let mut client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![vec![b'p', b'r', b'i', b'm']];
    let mut endpoint = Endpoint::client("127.0.0.1:0".parse().unwrap())?;
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
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<ReqwestMsg>(1024);
        let (mut send_stream, mut recv_stream) = match connection.open_bi().await {
            Ok(v) => v,
            Err(e) => {
                error!("open streams error: {}", e.to_string());
                continue;
            }
        };
        tokio::spawn(async move {
            let req_id = AtomicU64::new(0);
            let mut counter = 0;
            loop {
                select! {
                    req = receiver.recv() => {
                        match req {
                            Some(mut req) => {
                                let v = req_id.fetch_add(1, Ordering::SeqCst);
                                req.set_req_id(v);
                                // info!("{} send msg: {}",send_stream.id().0, v);
                                tokio::time::sleep(Duration::from_millis(2)).await;
                                if let Err(e) = ReqwestMsgIOUtil::send_msg(&req, &mut send_stream, Some(&mut counter)).await {
                                    error!("send msg error: {}", e.to_string());
                                    break;
                                }
                            },
                            None => {
                                error!("receiver closed.");
                                break;
                            }
                        }
                    },
                    resp = ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None) => {
                        match resp {
                            Ok(_) => {
                            },
                            Err(e) => {
                                error!("recv msg error: {}", e.to_string());
                                break;
                            }
                        }
                    }
                }
            }
            // warn!("{} send: {}", send_stream.id().0, counter);
        });
        sender_list.push(sender);
    }
    for i in 0..20000 {
        let sender = sender_list[i as usize % 8].clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let resource_id = fastrand::u16(..);
            let req =
                ReqwestMsg::with_resource_id_payload(resource_id, format!("{:06}", i).as_bytes());
            sender.send(req).await.unwrap();
        });
    }
    Ok(sender_list)
}
