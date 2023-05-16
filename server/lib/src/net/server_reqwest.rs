use std::{sync::Arc, task::Waker, time::Duration};

use anyhow::anyhow;
use dashmap::DashMap;
use futures_util::{StreamExt, FutureExt, pin_mut};
use quinn::NewConnection;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, debug};

use crate::{Result, entity::ReqwestMsg, net::{ReqwestMsgIOUtil, Operator}};

use super::{server::ServerConfig, ReqwestHandlerGenerator, ALPN_PRIM, ReqwestState};

pub struct ServerReqwest {
    config: Option<ServerConfig>,
    timeout: Duration,
    reqwest_caller_map: Arc<DashMap<u32, ReqwestState>>,
}

impl ServerReqwest {
    pub fn new(config: ServerConfig, timeout: Duration) -> Self {
        Self {
            config: Some(config),
            timeout,
            reqwest_caller_map: Arc::new(DashMap::new()),
        }
    }

    pub async fn run(&mut self, generator: ReqwestHandlerGenerator) -> Result<Arc<DashMap<u32, ReqwestState>>> {
        let ServerConfig {
            address,
            cert,
            key,
            max_connections,
            connection_idle_timeout,
            max_bi_streams,
        } = self.config.take().unwrap();
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        server_crypto.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let mut quinn_server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        quinn_server_config.concurrent_connections(max_connections as u32);
        quinn_server_config.use_retry(true);
        Arc::get_mut(&mut quinn_server_config.transport)
            .unwrap()
            .max_concurrent_bidi_streams(quinn::VarInt::from_u64(max_bi_streams as u64).unwrap())
            .max_idle_timeout(Some(quinn::IdleTimeout::from(
                quinn::VarInt::from_u64(connection_idle_timeout).unwrap(),
            )));
        let (endpoint, mut incoming) = quinn::Endpoint::server(quinn_server_config, address)?;
        let generator = Arc::new(generator);
        while let Some(conn) = incoming.next().await {
            let conn = conn.await?;
            info!(
                "new connection: {}",
                conn.connection.remote_address().to_string()
            );
            let generator = generator.clone();
            let caller_map = self.reqwest_caller_map.clone();
            let timeout = self.timeout;
            tokio::spawn(async move {
                let _ = Self::handle_new_connection(conn, generator, caller_map, timeout, max_bi_streams).await;
            });
        }
        endpoint.wait_idle().await;
        Ok(self.reqwest_caller_map.clone())
    }

    #[inline(always)]
    async fn handle_new_connection(
        mut conn: NewConnection,
        generator: Arc<ReqwestHandlerGenerator>,
        caller_map: Arc<DashMap<u32, ReqwestState>>,
        timeout: Duration,
        max_bi_streams: usize,
    ) -> Result<()> {
        let mut operator_list = Vec::with_capacity(max_bi_streams);
        let mut index = 0;
        let mut key = 0;
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
                    let mut handler = generator();
                    let (sender, mut receiver) = mpsc::channel::<(
                        ReqwestMsg,
                        Option<(u64, oneshot::Sender<Result<ReqwestMsg>>, Waker)>,
                    )>(16384);
                    let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
                    let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);
                    let (mut send_stream, mut recv_stream) = io_streams;

                    let resp_sender_map0 = Arc::new(DashMap::new());
                    let waker_map0 = Arc::new(DashMap::new());
                    let (tx, mut rx) = mpsc::channel::<u64>(4096);
                    let stream_id = recv_stream.id().0;
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
                                                ReqwestMsgIOUtil::send_msg(&req, &mut send_stream)
                                                    .await;
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
                    operator_list.push(Operator(index as u16, sender));
                    index += 1;
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
}
