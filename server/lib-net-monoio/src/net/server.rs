use std::{sync::{Arc, atomic::AtomicUsize}, time::Duration};
use std::sync::atomic::Ordering;
use anyhow::anyhow;

use futures::channel::mpsc;
use lib::{Result, entity::ReqwestMsg, net::{server::ServerConfig, ALPN_PRIM}};
use monoio::{net::TcpListener, io::{AsyncWriteRentExt, AsyncWriteRent}};
use monoio_rustls::{TlsAcceptor, TlsStream};
use tracing::{debug, error, info};
use async_trait::async_trait;
use futures::pin_mut;
use monoio::net::TcpStream;

pub type ReqwestHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewReqwestConnectionHandler> + Send + Sync + 'static>;
pub(self) type ReqwestHandlerGenerator0 =
    Box<dyn Fn() -> Box<dyn NewReqwestConnectionHandler0> + Send + Sync + 'static>;

#[async_trait]
pub trait NewReqwestConnectionHandler: Send + Sync + 'static {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()>;
}

#[async_trait]
pub(self) trait NewReqwestConnectionHandler0: Send + Sync + 'static {
    async fn handle(
        &mut self,
        msg_streams: TlsStream<TcpStream>,
    ) -> Result<()>;
}

pub(self) struct ServerReqwestTcp0 {
    config: Option<ServerConfig>,
}

impl ServerReqwestTcp0 {
    pub(self) fn new(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub(self) async fn run(&mut self, generator: ReqwestHandlerGenerator0) -> Result<()> {
        let ServerConfig {
            address,
            cert,
            key,
            connection_idle_timeout,
            max_connections,
            ..
        } = self.config.take().unwrap();
        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        config.alpn_protocols = ALPN_PRIM.iter().map(|&x| x.into()).collect();
        let connection_counter = Arc::new(AtomicUsize::new(0));
        let acceptor = TlsAcceptor::from(config);
        let listener = TcpListener::bind(address)?;
        while let Ok((stream, addr)) = listener.accept().await {
            let mut tls_stream = acceptor.accept(stream).await?;
            let handler = generator();
            let number = connection_counter.fetch_add(1, Ordering::SeqCst);
            if number > max_connections {
                tls_stream.write_all(b"too many connections.").await;
                tls_stream.flush().await?;
                tls_stream.shutdown().await?;
                error!("too many connections.");
                continue;
            }
            info!("new connection: {}", addr);
            let counter = connection_counter.clone();
            monoio::spawn(async move {
                let _ = Self::handle_new_connection(
                    tls_stream,
                    handler,
                    counter,
                    connection_idle_timeout,
                )
                .await;
            });
        }
        Ok(())
    }

    #[inline(always)]
    async fn handle_new_connection(
        stream: TlsStream<TcpStream>,
        mut handler: Box<dyn NewReqwestConnectionHandler>,
        connection_counter: Arc<AtomicUsize>,
        connection_idle_timeout: u64,
    ) -> Result<()> {
        // let idle_timeout = Duration::from_millis(connection_idle_timeout);
        // let io_operators =
        //     MsgIOTlsServerTimeoutWrapper::new(writer, reader, timeout, idle_timeout, None);
        // _ = handler.handle(io_operators).await;
        // debug!("connection closed.");
        // connection_counter.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}

// pub struct ServerReqwest {
//     server: ServerReqwest0,
//     timeout: Duration,
// }
//
// impl ServerReqwest {
//     pub fn new(config: ServerConfig, timeout: Duration) -> Self {
//         Self {
//             server: ServerReqwest0::new(config),
//             timeout,
//         }
//     }
//
//     pub async fn run(&mut self, generator: Arc<ReqwestHandlerGenerator>) -> Result<()> {
//         struct Generator0 {
//             generator: Arc<ReqwestHandlerGenerator>,
//             timeout: Duration,
//         }
//
//         #[async_trait]
//         impl NewReqwestConnectionHandler0 for Generator0 {
//             async fn handle(
//                 &mut self,
//                 msg_streams: (SendStream, RecvStream),
//                 client_caller: Option<Arc<ReqwestOperatorManager>>,
//             ) -> Result<Option<ReqwestOperator>> {
//                 let (mut send_stream, mut recv_stream) = msg_streams;
//                 let (sender, mut receiver) = mpsc::channel::<(
//                     ReqwestMsg,
//                     Option<(u64, Arc<ResponsePlaceholder>, Waker)>,
//                 )>(16384);
//                 let (msg_sender_outer, msg_receiver_outer) = mpsc::channel(16384);
//                 let (msg_sender_inner, mut msg_receiver_inner) = mpsc::channel(16384);
//
//                 let resp_waker_map0 = Arc::new(DashMap::new());
//                 let (tx, mut rx) = mpsc::channel::<u64>(4096);
//                 let stream_id = recv_stream.id().0;
//                 let sender_clone = sender.clone();
//                 let timeout = self.timeout;
//
//                 tokio::spawn(async move {
//                     let waker_map = resp_waker_map0.clone();
//
//                     let task1 = async {
//                         loop {
//                             match receiver.recv().await {
//                                 Some((req, external)) => match external {
//                                     // a request from server
//                                     Some((req_id, sender, waker)) => {
//                                         waker_map.insert(req_id, (waker, sender));
//                                         let res =
//                                             ReqwestMsgIOUtil::send_msg(&req, &mut send_stream)
//                                                 .await;
//                                         let tx = tx.clone();
//                                         tokio::spawn(async move {
//                                             tokio::time::sleep(timeout).await;
//                                             _ = tx.send(req_id).await;
//                                         });
//                                         if let Err(e) = res {
//                                             error!("send msg error: {}", e.to_string());
//                                             break;
//                                         }
//                                     }
//                                     // a response from server
//                                     None => {
//                                         if let Err(e) =
//                                             ReqwestMsgIOUtil::send_msg(&req, &mut send_stream).await
//                                         {
//                                             error!("send msg error: {}", e.to_string());
//                                             break;
//                                         }
//                                     }
//                                 },
//                                 None => {
//                                     debug!("receiver closed.");
//                                     _ = send_stream.finish().await;
//                                     break;
//                                 }
//                             }
//                         }
//                     }
//                     .fuse();
//
//                     let waker_map = resp_waker_map0.clone();
//
//                     let task2 = async {
//                         loop {
//                             match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
//                                 Ok(msg) => {
//                                     let req_id = msg.req_id();
//                                     // a response from client
//                                     if req_id ^ 0xF000_0000_0000_0000 == 0 {
//                                         match waker_map.remove(&req_id) {
//                                             Some(waker) => {
//                                                 waker.1 .0.wake();
//                                                 _ = waker.1 .1.set(Ok(msg));
//                                             }
//                                             None => {
//                                                 error!("req_id: {} not found.", req_id)
//                                             }
//                                         }
//                                     } else {
//                                         // a request from client
//                                         _ = msg_sender_outer.send(msg).await;
//                                     }
//                                 }
//                                 Err(e) => {
//                                     _ = recv_stream.stop(0u32.into());
//                                     debug!("recv msg error: {}", e.to_string());
//                                     break;
//                                 }
//                             }
//                         }
//                     }
//                     .fuse();
//
//                     let waker_map = resp_waker_map0;
//
//                     let task3 = async {
//                         loop {
//                             match rx.recv().await {
//                                 Some(timeout_id) => match waker_map.remove(&timeout_id) {
//                                     Some(waker) => {
//                                         waker.1 .0.wake();
//                                         _ = waker.1 .1.set(Err(anyhow!(
//                                             "{:06} timeout: {}",
//                                             stream_id,
//                                             timeout_id
//                                         )));
//                                     }
//                                     None => {}
//                                 },
//                                 None => {
//                                     debug!("rx closed.");
//                                     break;
//                                 }
//                             }
//                         }
//                     }
//                     .fuse();
//
//                     let task4 = async {
//                         loop {
//                             match msg_receiver_inner.recv().await {
//                                 Some(msg) => {
//                                     let res = sender_clone.send((msg, None)).await;
//                                     if let Err(e) = res {
//                                         error!("send msg error: {}", e.to_string());
//                                         break;
//                                     }
//                                 }
//                                 None => {
//                                     debug!("msg_receiver_inner closed.");
//                                     break;
//                                 }
//                             }
//                         }
//                     }
//                     .fuse();
//
//                     pin_mut!(task1, task2, task3, task4);
//
//                     loop {
//                         futures::select! {
//                             _ = task1 => {},
//                             _ = task2 => {},
//                             _ = task3 => {},
//                             _ = task4 => {},
//                             complete => {
//                                 break;
//                             }
//                         }
//                     }
//                 });
//
//                 let mut handler = (self.generator)();
//                 let caller = client_caller.unwrap();
//                 caller
//                     .push_operator(ReqwestOperator(stream_id as u16, sender))
//                     .await;
//                 handler.set_reqwest_caller(ReqwestCaller(caller));
//                 handler
//                     .handle((msg_sender_inner, msg_receiver_outer))
//                     .await
//                     .map_err(|e| {
//                         error!("handler error: {}", e.to_string());
//                         e
//                     })?;
//                 Ok(None)
//             }
//         }
//
//         let timeout = self.timeout;
//         let generator0: ReqwestHandlerGenerator0 = Box::new(move || {
//             Box::new(Generator0 {
//                 generator: generator.clone(),
//                 timeout,
//             })
//         });
//         self.server.run(generator0).await
//     }
// }