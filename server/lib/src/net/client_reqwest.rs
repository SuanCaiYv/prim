use std::{
    sync::{atomic::AtomicU64, Arc},
    task::Waker,
    time::Duration,
};

use anyhow::anyhow;
use dashmap::DashMap;
use futures_util::{pin_mut, FutureExt};
use quinn::{Connection, Endpoint};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

use crate::{entity::ReqwestMsg, net::ReqwestMsgIOUtil, Result};

use super::{client::ClientConfig, ReqwestHandlerGenerator, ALPN_PRIM, ReqwestState, Operator};

pub struct ReqwestClient {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    req_timeout: Duration,
}

impl ReqwestClient {
    pub fn new(config: ClientConfig, timeout: Duration) -> Self {
        Self {
            config: Some(config),
            endpoint: None,
            connection: None,
            req_timeout: timeout,
        }
    }

    pub async fn build(&mut self, generator: Arc<ReqwestHandlerGenerator>) -> Result<ReqwestState> {
        let ClientConfig {
            remote_address,
            ipv4_type,
            domain,
            cert,
            keep_alive_interval,
            max_bi_streams,
        } = self.config.take().unwrap();
        let default_address = if ipv4_type {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let mut roots = rustls::RootCertStore::empty();
        roots.add(&cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut endpoint = Endpoint::client(default_address)?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut client_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .keep_alive_interval(Some(keep_alive_interval));
        endpoint.set_default_client_config(client_config);
        let new_connection = endpoint
            .connect(remote_address, domain.as_str())
            .unwrap()
            .await
            .map_err(|e| anyhow!("failed to connect: {:?}", e))?;
        let quinn::NewConnection { connection, .. } = new_connection;
        let mut operator_list = Vec::with_capacity(max_bi_streams as usize);

        for i in 0..max_bi_streams {
            let (sender, mut receiver) = mpsc::channel::<(
                ReqwestMsg,
                Option<(u64, oneshot::Sender<Result<ReqwestMsg>>, Waker)>,
            )>(16384);
            let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
            let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);
            let (mut send_stream, mut recv_stream) = match connection.open_bi().await {
                Ok(v) => v,
                Err(e) => {
                    error!("open streams error: {}", e.to_string());
                    continue;
                }
            };

            let resp_sender_map0 = Arc::new(DashMap::new());
            let waker_map0 = Arc::new(DashMap::new());
            let (tx, mut rx) = mpsc::channel::<u64>(4096);
            let stream_id = recv_stream.id().0;
            let timeout = self.req_timeout;
            let sender_clone = sender.clone();

            tokio::spawn(async move {
                let resp_sender_map = resp_sender_map0.clone();
                let waker_map = waker_map0.clone();

                let task1 = async {
                    loop {
                        match receiver.recv().await {
                            Some((req, external)) => match external {
                                Some((req_id, sender, waker)) => {
                                    resp_sender_map.insert(req_id, sender);
                                    waker_map.insert(req_id, waker);
                                    let res =
                                        ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await;
                                    let tx = tx.clone();
                                    tokio::spawn(async move {
                                        tokio::time::sleep(timeout).await;
                                        _ = tx.send(req_id).await;
                                    });
                                    if let Err(e) = res {
                                        error!("send msg error: {}", e.to_string());
                                        break;
                                    }
                                }
                                None => {}
                            },
                            None => {
                                debug!("receiver closed.");
                                _ = send_stream.finish().await;
                                break;
                            }
                        }
                    }
                }
                .fuse();

                let resp_sender_map = resp_sender_map0.clone();
                let waker_map = waker_map0.clone();

                let task2 = async {
                    loop {
                        match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                            Ok(msg) => {
                                let req_id = msg.req_id();
                                if req_id ^ 0xF000_0000_0000_0000 == 0 {
                                    msg_sender_outer.send(msg).await;
                                } else {
                                    match resp_sender_map.remove(&req_id) {
                                        Some(sender) => {
                                            _ = sender.1.send(Ok(msg));
                                        }
                                        None => {
                                            error!("req_id: {} not found.", req_id)
                                        }
                                    }
                                    match waker_map.remove(&req_id) {
                                        Some(waker) => {
                                            waker.1.wake();
                                        }
                                        None => {
                                            error!("req_id: {} not found.", req_id)
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                _ = recv_stream.stop(0u32.into());
                                debug!("recv msg error: {}", e.to_string());
                                break;
                            }
                        }
                    }
                }
                .fuse();

                let resp_sender_map = resp_sender_map0;
                let waker_map = waker_map0;

                let task3 = async {
                    loop {
                        match rx.recv().await {
                            Some(timeout_id) => {
                                match resp_sender_map.remove(&timeout_id) {
                                    Some(sender) => {
                                        _ = sender.1.send(Err(anyhow!(
                                            "{:02} timeout: {}",
                                            stream_id,
                                            timeout_id
                                        )));
                                    }
                                    None => {}
                                }
                                match waker_map.remove(&timeout_id) {
                                    Some(waker) => {
                                        waker.1.wake();
                                    }
                                    None => {}
                                }
                            }
                            None => {
                                debug!("rx closed.");
                                break;
                            }
                        }
                    }
                }
                .fuse();

                let task4 = async {
                    loop {
                        match msg_receiver_inner.recv().await {
                            Some(msg) => {
                                let res = sender_clone.send((msg, None)).await;
                                if let Err(e) = res {
                                    error!("send msg error: {}", e.to_string());
                                    break;
                                }
                            }
                            None => {
                                debug!("msg_receiver_inner closed.");
                                break;
                            }
                        }
                    }
                }
                .fuse();

                pin_mut!(task1, task2, task3, task4);

                loop {
                    futures::select! {
                        _ = task1 => {},
                        _ = task2 => {},
                        _ = task3 => {},
                        _ = task4 => {},
                        complete => {
                            break;
                        }
                    }
                }
            });

            let mut handler = generator();
            tokio::spawn(async move {
                handler
                    .handle((msg_sender_inner, msg_receiver_outer))
                    .await
                    .map_err(|e| {
                        error!("handler error: {}", e.to_string());
                        e
                    })?;
                Result::<()>::Ok(())
            });
            operator_list.push(Operator(i as u16, sender));
        }
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        Ok(ReqwestState {
            req_id: AtomicU64::new(0),
            operator_list,
        })
    }
}

impl Drop for ReqwestClient {
    fn drop(&mut self) {
        self.connection
            .as_ref()
            .unwrap()
            .close(0u32.into(), b"it's time to say goodbye.");
    }
}
