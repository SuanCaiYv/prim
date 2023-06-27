use std::cell::UnsafeCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Waker};
use anyhow::anyhow;
use byteorder::BigEndian;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::pin_mut;
use lib::{entity::ReqwestMsg, Result, error::CrashError};
use monoio::{io::{AsyncWriteRent, AsyncWriteRentExt, AsyncReadRentExt}, net::TcpStream};
use monoio_rustls::TlsStream;
use tracing::{debug, error};
use lib::net::GenericParameter;

pub mod server;

// pub struct ReqwestMsgIOUtil {}
//
// impl ReqwestMsgIOUtil {
//     #[inline(always)]
//     pub async fn send_msg(msg: ReqwestMsg, stream: &mut TlsStream<TcpStream>) -> Result<ReqwestMsg> {
//         let (res, msg) = stream.write_all(msg.0).await;
//         match res {
//             Err(e) => {
//                 _ = stream.shutdown().await;
//                 debug!("write stream error: {:?}", e);
//                 Err(anyhow!(CrashError::ShouldCrash(
//                 "write stream error.".to_string()
//             )))
//             }
//             Ok(_) => {
//                 Ok(ReqwestMsg(msg))
//             }
//         }
//     }
//
//     #[inline(always)]
//     pub async fn recv_msg(
//         stream: &mut TlsStream<TcpStream>,
//     ) -> Result<ReqwestMsg> {
//         let mut len_buf: [u8; 2] = [0u8; 2];
//         match stream.read_exact(&mut len_buf[..]).await {
//             Ok(_) => {}
//             Err(_) => {
//                 return Err(anyhow!(CrashError::ShouldCrash(
//                     "read stream error.".to_string()
//                 )));
//             }
//         }
//         let len = BigEndian::read_u16(&len_buf[..]);
//         let mut msg = ReqwestMsg::pre_alloc(len);
//         match stream.read_exact(msg.body_mut()).await {
//             Ok(_) => {}
//             Err(_) => {
//                 return Err(anyhow!(CrashError::ShouldCrash(
//                     "read stream error.".to_string()
//                 )));
//             }
//         }
//         Ok(msg)
//     }
// }
//
// pub struct ReqwestMsgIOWrapper {
//     pub(self) send_channel: Option<mpsc::Sender<ReqwestMsg>>,
//     pub(self) recv_channel: Option<mpsc::Receiver<ReqwestMsg>>,
// }
//
// impl ReqwestMsgIOWrapper {
//     pub fn new(mut stream: TlsStream<TcpStream>) -> Self {
//         let (send_sender, mut send_receiver): (
//             mpsc::Sender<ReqwestMsg>,
//             mpsc::Receiver<ReqwestMsg>,
//         ) = mpsc::channel(16384);
//         let (mut recv_sender, recv_receiver): (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) =
//             mpsc::channel(16384);
//         monoio::spawn(async {
//             let task1 = async {
//                 loop {
//                     match ReqwestMsgIOUtil::recv_msg(&mut stream).await {
//                         Ok(msg) => {
//                             if let Err(e) = recv_sender.send(msg).await {
//                                 error!("send msg error: {}", e.to_string());
//                                 break;
//                             }
//                         }
//                         Err(_) => {
//                             stream.shutdown().await;
//                             break;
//                         }
//                     }
//                 }
//             }
//                 .fuse();
//
//             let task2 = async {
//                 loop {
//                     match send_receiver.recv().await {
//                         Some(msg) => {
//                             if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut stream).await
//                             {
//                                 error!("send msg error: {}", e.to_string());
//                                 break;
//                             }
//                         }
//                         None => {
//                             _ = stream.shutdown().await;
//                             break;
//                         }
//                     }
//                 }
//             }
//                 .fuse();
//
//             pin_mut!(task1, task2);
//
//             loop {
//                 futures::select! {
//                     _ = task1 => {},
//                     _ = task2 => {},
//                     complete => {
//                         break;
//                     }
//                 }
//             }
//         });
//         #[cfg(feature = "no-select")]
//         tokio::spawn(async move {
//             loop {
//                 match send_receiver.recv().await {
//                     Some(msg) => {
//                         if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await {
//                             error!("send msg error: {:?}", e);
//                             break;
//                         }
//                     }
//                     None => {
//                         _ = send_stream.finish().await;
//                         break;
//                     }
//                 }
//             }
//         });
//         #[cfg(feature = "no-select")]
//         tokio::spawn(async move {
//             loop {
//                 match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None, 1).await {
//                     Ok(msg) => {
//                         if let Err(e) = recv_sender.send(msg).await {
//                             error!("send msg error: {}", e.to_string());
//                             break;
//                         }
//                     }
//                     Err(_) => {
//                         _ = recv_stream.stop(0u32.into());
//                         break;
//                     }
//                 }
//             }
//         });
//         Self {
//             send_channel: Some(send_sender),
//             recv_channel: Some(recv_receiver),
//         }
//     }
//
//     pub fn channels(&mut self) -> (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) {
//         let send = self.send_channel.take().unwrap();
//         let recv = self.recv_channel.take().unwrap();
//         (send, recv)
//     }
// }
//
// pub(super) struct ResponsePlaceholder {
//     value: UnsafeCell<Option<Result<ReqwestMsg>>>,
// }
//
// impl ResponsePlaceholder {
//     pub fn new() -> Self {
//         Self {
//             value: UnsafeCell::new(None),
//         }
//     }
//
//     pub fn set(&self, new_value: Result<ReqwestMsg>) {
//         unsafe {
//             (&mut (*self.value.get())).replace(new_value);
//         }
//     }
//
//     pub fn get(&self) -> Option<Result<ReqwestMsg>> {
//         unsafe { (&mut (*self.value.get())).take() }
//     }
// }
//
// unsafe impl Send for ResponsePlaceholder {}
//
// unsafe impl Sync for ResponsePlaceholder {}
//
// pub(self) struct ReqwestOperator(
//     pub(crate) u16,
//     pub(crate) mpsc::Sender<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>,
// );
//
// pub struct Reqwest {
//     req_id: u64,
//     sender_task_done: bool,
//     req: Option<ReqwestMsg>,
//     operator_sender:
//     Option<mpsc::Sender<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>>,
//     sender_task: Option<BoxFuture<'static, Result<()>>>,
//     resp_receiver: Arc<ResponsePlaceholder>,
//     load_counter: Arc<AtomicU64>,
// }
//
// impl Unpin for Reqwest {}
//
// impl Future for Reqwest {
//     type Output = Result<ReqwestMsg>;
//
//     /// the request will not sent until the future is polled.
//     ///
//     /// note: the request may loss by network crowded, for example: if there are to much packets arrived at server endpoint,
//     /// the server will drop some packets, although we have some mechanism to try best to get all request.
//     ///
//     /// and we also set a timeout notification, if the request is not responded in some mill-seconds.
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         if !self.sender_task_done {
//             match self.sender_task.as_mut() {
//                 Some(task) => {
//                     match task.as_mut().poll(cx) {
//                         std::task::Poll::Ready(_) => {
//                             self.sender_task_done = true;
//                         }
//                         std::task::Poll::Pending => {
//                             return std::task::Poll::Pending;
//                         }
//                     };
//                 }
//                 None => {
//                     let req = self.req.take().unwrap();
//                     let req_id = self.req_id;
//                     let waker = cx.waker().clone();
//                     let operator_sender = self.operator_sender.take().unwrap();
//                     let tx = self.resp_receiver.clone();
//                     let task = async move {
//                         if let Err(e) = operator_sender.send((req, Some((req_id, tx, waker)))).await
//                         {
//                             error!("send req error: {}", e.to_string());
//                             return Err(anyhow!(e.to_string()));
//                         }
//                         Ok(())
//                     };
//                     let task: BoxFuture<Result<()>> = Box::pin(task);
//                     self.sender_task = Some(task);
//                     match self.sender_task.as_mut().unwrap().as_mut().poll(cx) {
//                         std::task::Poll::Ready(_) => {
//                             self.sender_task_done = true;
//                         }
//                         std::task::Poll::Pending => {
//                             return std::task::Poll::Pending;
//                         }
//                     };
//                 }
//             };
//         }
//         match self.resp_receiver.get() {
//             Some(resp) => {
//                 self.load_counter.fetch_sub(1, Ordering::SeqCst);
//                 std::task::Poll::Ready(resp)
//             }
//             None => std::task::Poll::Pending,
//         }
//     }
// }
//
// pub struct ReqwestOperatorManager {
//     target_mask: u64,
//     pub(self) req_id: AtomicU64,
//     pub(self) load_list: UnsafeCell<Vec<Arc<AtomicU64>>>,
//     pub(self) operator_list: UnsafeCell<Vec<ReqwestOperator>>,
// }
//
// unsafe impl Send for ReqwestOperatorManager {}
//
// unsafe impl Sync for ReqwestOperatorManager {}
//
// impl ReqwestOperatorManager {
//     fn new() -> Self {
//         Self {
//             target_mask: 0,
//             req_id: AtomicU64::new(0),
//             load_list: UnsafeCell::new(Vec::new()),
//             operator_list: UnsafeCell::new(Vec::new()),
//         }
//     }
//
//     fn new_directly(operator_list: Vec<ReqwestOperator>) -> Self {
//         let load_list = operator_list
//             .iter()
//             .map(|_| Arc::new(AtomicU64::new(0)))
//             .collect::<Vec<_>>();
//         Self {
//             target_mask: 0,
//             req_id: AtomicU64::new(0),
//             load_list: UnsafeCell::new(load_list),
//             operator_list: UnsafeCell::new(operator_list),
//         }
//     }
//
//     async fn push_operator(&self, operator: ReqwestOperator) {
//         let operator_list = unsafe { &mut *self.operator_list.get() };
//         operator_list.push(operator);
//         let load_list = unsafe { &mut *self.load_list.get() };
//         load_list.push(Arc::new(AtomicU64::new(0)));
//     }
//
//     pub fn call(&self, mut req: ReqwestMsg) -> Reqwest {
//         let mut min_index = 0;
//         let mut min_load = u64::MAX;
//         for (i, load) in unsafe { &mut *self.load_list.get() }.iter().enumerate() {
//             let load_val = load.load(Ordering::SeqCst);
//             if load_val < min_load {
//                 min_load = load_val;
//                 min_index = i;
//             }
//         }
//         (unsafe { &*self.load_list.get() })[min_index].fetch_add(1, Ordering::SeqCst);
//         let req_id = self.req_id.fetch_add(1, Ordering::SeqCst);
//         let operator = &(unsafe { &*self.operator_list.get() })[min_index];
//         let req_sender = operator.1.clone();
//         let resp_receiver = Arc::new(ResponsePlaceholder::new());
//         req.set_req_id(req_id | self.target_mask);
//         Reqwest {
//             req_id,
//             req: Some(req),
//             sender_task: None,
//             resp_receiver,
//             sender_task_done: false,
//             operator_sender: Some(req_sender),
//             load_counter: (unsafe { &*self.load_list.get() })[min_index].clone(),
//         }
//     }
// }
//
// impl GenericParameter for ReqwestOperatorManager {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
//
//     fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
//         self
//     }
// }
