use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    task::Waker,
    time::Duration,
};

use anyhow::anyhow;
use dashmap::DashMap;
use futures_util::{future::BoxFuture, pin_mut, FutureExt};
use quinn::{Connection, Endpoint};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

use crate::{entity::ReqwestMsg, net::ReqwestMsgIOUtil, Result};

use super::{client::ClientConfig, ALPN_PRIM};

pub struct ReqwestClient {
    config: Option<ClientConfig>,
    endpoint: Option<Endpoint>,
    connection: Option<Connection>,
    req_timeout: Duration,
}

pub(self) struct Operator(
    AtomicU64,
    mpsc::Sender<(u64, ReqwestMsg, oneshot::Sender<Result<ReqwestMsg>>, Waker)>,
    u16,
);

pub struct Reqwest {
    req_id: u64,
    sender_task_done: bool,
    req: Option<ReqwestMsg>,
    operator_sender: Option<
        tokio::sync::mpsc::Sender<(
            u64,
            ReqwestMsg,
            tokio::sync::oneshot::Sender<Result<ReqwestMsg>>,
            Waker,
        )>,
    >,
    sender_task: Option<BoxFuture<'static, Result<()>>>,
    resp_receiver: Option<tokio::sync::oneshot::Receiver<Result<ReqwestMsg>>>,
}

pub struct ReqwestState {
    index: AtomicUsize,
    operator_list: Vec<Operator>,
}

impl ReqwestState {
    pub fn call(&self, mut req: ReqwestMsg) -> Reqwest {
        let index = self.index.fetch_add(1, Ordering::SeqCst);
        let operator = &self.operator_list[index % self.operator_list.len()];
        let req_id = operator.0.fetch_add(1, Ordering::SeqCst);
        let req_sender = operator.1.clone();
        req.set_req_id(req_id);
        Reqwest {
            req_id,
            req: Some(req),
            sender_task: None,
            resp_receiver: None,
            sender_task_done: false,
            operator_sender: Some(req_sender),
        }
    }
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

    pub async fn build(&mut self) -> Result<ReqwestState> {
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
            let req_id = AtomicU64::new(0);
            let (sender, mut receiver) = tokio::sync::mpsc::channel::<(
                ReqwestMsg,
                Option<(u64, oneshot::Sender<Result<ReqwestMsg>>, Waker)>,
            )>(1024);
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

            tokio::spawn(async move {
                let resp_sender_map = resp_sender_map0.clone();
                let waker_map = waker_map0.clone();

                let task1 = async {
                    loop {
                        match receiver.recv().await {
                            Some((req_id, req, sender, waker)) => {
                                resp_sender_map.insert(req_id, sender);
                                waker_map.insert(req_id, waker);
                                let res = ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await;
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
                            Ok(resp) => {
                                let req_id = resp.req_id();
                                match resp_sender_map.remove(&req_id) {
                                    Some(sender) => {
                                        _ = sender.1.send(Ok(resp));
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

                pin_mut!(task1, task2, task3);

                loop {
                    futures::select! {
                        _ = task1 => {},
                        _ = task2 => {},
                        _ = task3 => {},
                        complete => {
                            break;
                        }
                    }
                }
            });
            operator_list.push(Operator(req_id, sender, i as u16));
        }
        self.endpoint = Some(endpoint);
        self.connection = Some(connection);
        Ok(ReqwestState {
            index: AtomicUsize::new(0),
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
