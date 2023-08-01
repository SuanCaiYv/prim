use std::{time::Duration, task::Waker, sync::Arc};

use futures::{pin_mut, FutureExt};
use lib::{Result, net::{client::ClientConfig, ALPN_PRIM}, entity::{ReqwestMsg, ReqwestResourceID}, util::map::LocalMap};
use local_sync::mpsc;
use monoio::{net::TcpStream, io::{Splitable, AsyncWriteRent}};
use monoio_rustls::TlsConnector;
use tracing::{debug, error};
use anyhow::anyhow;

use crate::net::ReqwestMsgIOUtil;

use super::{ReqwestOperatorManager, ResponsePlaceholder, ReqwestOperator};

pub struct ClientReqwestTcp {
    config: Option<ClientConfig>,
    timeout: Duration,
}

impl ClientReqwestTcp {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        ClientReqwestTcp {
            config: Some(config),
            timeout,
        }
    }

    pub async fn build(&mut self) -> Result<ReqwestOperatorManager> {
        let ClientConfig {
            remote_address,
            domain,
            cert,
            keep_alive_interval,
            ..
        } = self.config.take().unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let connector = TlsConnector::from(Arc::new(client_crypto));
        let stream = TcpStream::connect(remote_address).await?;
        let domain = rustls::ServerName::try_from(domain.as_str()).unwrap();
        let stream = connector.connect(domain, stream).await?;

        let (sender, mut receiver) =
            mpsc::bounded::channel::<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>(16384);
        let (inner_sender, mut inner_receiver) = mpsc::bounded::channel(1024);

        let resp_waker_map0 = Arc::new(LocalMap::new());
        let (tx, mut rx) = mpsc::bounded::channel::<u64>(4096);
        let mut ticker = monoio::time::interval(keep_alive_interval);
        let tick_sender = inner_sender.clone();
        let timeout = self.timeout;

        monoio::spawn(async move {
            let (mut recv_stream, mut send_stream) = stream.into_split();
            let resp_waker_map = resp_waker_map0.clone();

            let task1 = async {
                loop {
                    match inner_receiver.recv().await {
                        Some(msg) => {
                            let res = ReqwestMsgIOUtil::send_msgc(msg, &mut send_stream).await;
                            if let Err(e) = res {
                                error!("send msg error: {}", e.to_string());
                                inner_receiver.close();
                                break;
                            }
                        }
                        None => {
                            debug!("receiver closed.");
                            _ = send_stream.shutdown().await;
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                loop {
                    match receiver.recv().await {
                        Some((req, external)) => match external {
                            // a request from client
                            Some((req_id, sender, waker)) => {
                                resp_waker_map.insert(req_id, (waker, sender));
                                let res = inner_sender.send(req).await;
                                let tx = tx.clone();
                                monoio::spawn(async move {
                                    monoio::time::sleep(timeout).await;
                                    _ = tx.send(req_id).await;
                                });
                                if let Err(_) = res {
                                    receiver.close();
                                    break;
                                }
                            }
                            // a response from client
                            None => {
                                if let Err(_) = inner_sender.send(req).await {
                                    receiver.close();
                                    break;
                                }
                            }
                        },
                        None => {
                            drop(inner_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            let resp_waker_map = resp_waker_map0.clone();

            let task3 = async {
                loop {
                    match ReqwestMsgIOUtil::recv_msgc(&mut recv_stream).await {
                        Ok(msg) => {
                            if msg.resource_id() == ReqwestResourceID::Pong {
                                continue;
                            }
                            let req_id = msg.req_id();
                            // a request from server
                            if req_id & 0xF000_0000_0000_0000 != 0 {
                                todo!("server request")
                            } else {
                                // a response from server
                                match resp_waker_map.remove(&req_id) {
                                    Some(waker) => {
                                        waker.0.wake();
                                        _ = waker.1.set(Ok(msg));
                                    }
                                    None => {
                                        error!("req_id: {} not found.", req_id)
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            debug!("recv msg error: {}", e.to_string());
                            break;
                        }
                    }
                }
            }
            .fuse();

            let waker_map = resp_waker_map0;

            let task4 = async {
                loop {
                    match rx.recv().await {
                        Some(timeout_id) => match waker_map.remove(&timeout_id) {
                            Some(waker) => {
                                waker.0.wake();
                                _ = waker.1.set(Err(anyhow!("timeout: {}", timeout_id)));
                            }
                            None => {}
                        },
                        None => {
                            debug!("rx closed.");
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task5 = async move {
                loop {
                    ticker.tick().await;
                    let msg = ReqwestMsg::with_resource_id_payload(ReqwestResourceID::Ping, b"");
                    if let Err(e) = tick_sender.send(msg).await {
                        error!("send msg error: {:?}", e);
                        break;
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2, task3, task4, task5);

            loop {
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    _ = task3 => {},
                    _ = task4 => {},
                    _ = task5 => {},
                    complete => {
                        break;
                    }
                }
            }
        });

        let operator_manager =
            ReqwestOperatorManager::new_directly(vec![ReqwestOperator(1, sender)]);
        Ok(operator_manager)
    }
}
