use std::{
    cell::UnsafeCell,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use futures::{future::LocalBoxFuture, pin_mut, Future, FutureExt};
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    error::CrashError,
    net::InnerStates,
    Result,
};
use local_sync::mpsc;
use monoio::{
    io::{
        AsyncReadRentExt, AsyncWriteRent, AsyncWriteRentExt, OwnedReadHalf, OwnedWriteHalf,
        Splitable,
    },
    net::TcpStream,
    time::{Instant, Sleep},
};
use monoio_rustls::{ClientTlsStream, ServerTlsStream};
use tracing::{debug, error};

pub mod client;
pub mod server;

pub type ReqwestHandlerMap = Arc<AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>>>;

#[async_trait(? Send)]
pub trait ReqwestHandler: 'static {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg>;
}

pub(super) struct ResponsePlaceholder {
    value: UnsafeCell<Option<Result<ReqwestMsg>>>,
}

impl ResponsePlaceholder {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    pub fn set(&self, new_value: Result<ReqwestMsg>) {
        unsafe {
            (&mut (*self.value.get())).replace(new_value);
        }
    }

    pub fn get(&self) -> Option<Result<ReqwestMsg>> {
        unsafe { (&mut (*self.value.get())).take() }
    }
}

unsafe impl Send for ResponsePlaceholder {}
unsafe impl Sync for ResponsePlaceholder {}

pub(self) struct ReqwestOperator(
    pub(crate) u16,
    pub(crate) mpsc::bounded::Tx<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>,
);

pub struct Reqwest {
    req_id: u64,
    sender_task_done: bool,
    req: Option<ReqwestMsg>,
    operator_sender:
        Option<mpsc::bounded::Tx<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>>,
    sender_task: Option<LocalBoxFuture<'static, Result<()>>>,
    resp_receiver: Arc<ResponsePlaceholder>,
    load_counter: Arc<AtomicU64>,
}

impl Unpin for Reqwest {}

impl Future for Reqwest {
    type Output = Result<ReqwestMsg>;

    /// the request will not sent until the future is polled.
    ///
    /// note: the request may loss by network crowded, for example: if there are to much packets arrived at server endpoint,
    /// the server will drop some packets, although we have some mechanism to try best to get all request.
    ///
    /// and we also set a timeout notification, if the request is not responded in some mill-seconds.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.sender_task_done {
            match self.sender_task.as_mut() {
                Some(task) => {
                    match task.as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    };
                }
                None => {
                    let req = self.req.take().unwrap();
                    let req_id = self.req_id;
                    let waker = cx.waker().clone();
                    let operator_sender = self.operator_sender.take().unwrap();
                    let tx = self.resp_receiver.clone();
                    let task = async move {
                        if let Err(_e) = operator_sender.send((req, Some((req_id, tx, waker)))).await
                        {
                            error!("rx closed.");
                            return Err(anyhow!("rx closed."));
                        }
                        Ok(())
                    };
                    let task: LocalBoxFuture<Result<()>> = Box::pin(task);
                    self.sender_task = Some(task);
                    match self.sender_task.as_mut().unwrap().as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    };
                }
            };
        }
        match self.resp_receiver.get() {
            Some(resp) => {
                self.load_counter.fetch_sub(1, Ordering::AcqRel);
                Poll::Ready(resp)
            }
            None => Poll::Pending,
        }
    }
}

pub struct ReqwestOperatorManager {
    target_mask: u64,
    pub(self) req_id: AtomicU64,
    pub(self) load_list: UnsafeCell<Vec<Arc<AtomicU64>>>,
    pub(self) operator_list: UnsafeCell<Vec<ReqwestOperator>>,
}

unsafe impl Send for ReqwestOperatorManager {}
unsafe impl Sync for ReqwestOperatorManager {}

impl ReqwestOperatorManager {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            target_mask: 0,
            req_id: AtomicU64::new(0),
            load_list: UnsafeCell::new(Vec::new()),
            operator_list: UnsafeCell::new(Vec::new()),
        }
    }

    fn new_directly(operator_list: Vec<ReqwestOperator>) -> Self {
        let load_list = operator_list
            .iter()
            .map(|_| Arc::new(AtomicU64::new(0)))
            .collect::<Vec<_>>();
        Self {
            target_mask: 0,
            req_id: AtomicU64::new(0),
            load_list: UnsafeCell::new(load_list),
            operator_list: UnsafeCell::new(operator_list),
        }
    }

    #[allow(dead_code)]
    async fn push_operator(&self, operator: ReqwestOperator) {
        let operator_list = unsafe { &mut *self.operator_list.get() };
        operator_list.push(operator);
        let load_list = unsafe { &mut *self.load_list.get() };
        load_list.push(Arc::new(AtomicU64::new(0)));
    }

    pub fn call(&self, mut req: ReqwestMsg) -> Reqwest {
        let mut min_index = 0;
        let mut min_load = u64::MAX;
        for (i, load) in unsafe { &mut *self.load_list.get() }.iter().enumerate() {
            let load_val = load.load(Ordering::Acquire);
            if load_val < min_load {
                min_load = load_val;
                min_index = i;
            }
        }
        (unsafe { &*self.load_list.get() })[min_index].fetch_add(1, Ordering::AcqRel);
        let req_id = self.req_id.fetch_add(1, Ordering::AcqRel);
        let operator = &(unsafe { &*self.operator_list.get() })[min_index];
        let req_sender = operator.1.clone();
        let resp_receiver = Arc::new(ResponsePlaceholder::new());
        let req_id = req_id | self.target_mask;
        req.set_req_id(req_id);
        Reqwest {
            req_id,
            req: Some(req),
            sender_task: None,
            resp_receiver,
            sender_task_done: false,
            operator_sender: Some(req_sender),
            load_counter: (unsafe { &*self.load_list.get() })[min_index].clone(),
        }
    }
}

#[derive(Clone)]
pub struct TimerSetter {
    sender: mpsc::bounded::Tx<Instant>,
}

impl TimerSetter {
    pub fn new(sender: mpsc::bounded::Tx<Instant>) -> Self {
        Self { sender }
    }

    pub async fn set(&self, new_timeout: Instant) {
        _ = self.sender.send(new_timeout).await;
    }
}

pub struct SharedTimer<'a> {
    timer: Pin<Box<Sleep>>,
    task: LocalBoxFuture<'a, ()>,
    set_sender: mpsc::bounded::Tx<Instant>,
    set_receiver: mpsc::bounded::Rx<Instant>,
}

impl<'a> SharedTimer<'a> {
    pub fn new(default_timeout: Duration, callback: impl Future<Output = ()> + 'static) -> Self {
        let timer = monoio::time::sleep(default_timeout);
        let (set_sender, set_receiver) = mpsc::bounded::channel(256);
        Self {
            timer: Box::pin(timer),
            task: Box::pin(callback),
            set_sender,
            set_receiver,
        }
    }

    pub fn setter(&self) -> TimerSetter {
        TimerSetter::new(self.set_sender.clone())
    }
}

impl Unpin for SharedTimer<'_> {}

impl<'a> Future for SharedTimer<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut new_timeout = None;
        loop {
            match self.set_receiver.poll_recv(cx) {
                Poll::Pending => {
                    break;
                }
                Poll::Ready(Some(timeout)) => {
                    new_timeout = Some(timeout);
                }
                Poll::Ready(None) => {
                    return Poll::Ready(());
                }
            }
        }
        match new_timeout {
            Some(timeout) => {
                self.timer.as_mut().reset(timeout);
            }
            None => {}
        }
        match self.timer.as_mut().poll(cx) {
            Poll::Ready(_) => match self.task.as_mut().poll(cx) {
                Poll::Ready(_) => Poll::Ready(()),
                Poll::Pending => Poll::Pending,
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct ReqwestMsgIOUtil {}

impl ReqwestMsgIOUtil {
    #[inline(always)]
    pub async fn send_msgs(
        msg: ReqwestMsg,
        stream: &mut OwnedWriteHalf<ServerTlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let (res, msg) = stream.write_all(msg.0).await;
        match res {
            Err(e) => {
                _ = stream.shutdown().await;
                debug!("write stream error: {:?}", e);
                Err(anyhow!(CrashError::ShouldCrash(
                    "write stream error.".to_string()
                )))
            }
            Ok(_size) => Ok(ReqwestMsg(msg)),
        }
    }

    #[inline(always)]
    pub async fn recv_msgs(
        stream: &mut OwnedReadHalf<ServerTlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let len_buf: Box<[u8; 2]> = Box::new([0u8; 2]);
        let (res, len_buf) = stream.read_exact(len_buf).await;
        match res {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let len = BigEndian::read_u16(len_buf.as_ref());
        let mut msg = ReqwestMsg::pre_alloc(len);
        let body = msg.body_mut().to_owned();
        let (res, body) = stream.read_exact(body).await;
        match res {
            Ok(_) => {
                msg.set_body(body.as_slice());
            }
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        Ok(msg)
    }

    #[inline(always)]
    pub async fn send_msgc(
        msg: ReqwestMsg,
        stream: &mut OwnedWriteHalf<ClientTlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let (res, msg) = stream.write_all(msg.0).await;
        match res {
            Err(e) => {
                _ = stream.shutdown().await;
                debug!("write stream error: {:?}", e);
                Err(anyhow!(CrashError::ShouldCrash(
                    "write stream error.".to_string()
                )))
            }
            Ok(_size) => Ok(ReqwestMsg(msg)),
        }
    }

    #[inline(always)]
    pub async fn recv_msgc(
        stream: &mut OwnedReadHalf<ClientTlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let len_buf: Box<[u8; 2]> = Box::new([0u8; 2]);
        let (res, len_buf) = stream.read_exact(len_buf).await;
        match res {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let len = BigEndian::read_u16(len_buf.as_ref());
        let mut msg = ReqwestMsg::pre_alloc(len);
        let body = msg.body_mut().to_owned();
        let (res, body) = stream.read_exact(body).await;
        match res {
            Ok(_) => {
                msg.set_body(body.as_slice());
            }
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        Ok(msg)
    }
}

pub struct ReqwestMsgIOWrapper {
    pub(self) send_channel: Option<mpsc::bounded::Tx<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::bounded::Rx<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapper {
    pub fn new(stream: ServerTlsStream<TcpStream>, idle_timeout: Duration) -> Self {
        let (send_sender, mut send_receiver): (
            mpsc::bounded::Tx<ReqwestMsg>,
            mpsc::bounded::Rx<ReqwestMsg>,
        ) = mpsc::bounded::channel(16384);
        let (recv_sender, recv_receiver): (
            mpsc::bounded::Tx<ReqwestMsg>,
            mpsc::bounded::Rx<ReqwestMsg>,
        ) = mpsc::bounded::channel(16384);
        let send_sender0 = send_sender.clone();
        let close_sender = send_sender.clone();
        monoio::spawn(async move {
            let (mut recv_stream, mut send_stream) = stream.into_split();
            let timer = SharedTimer::new(idle_timeout, async move {
                let msg =
                    ReqwestMsg::with_resource_id_payload(ReqwestResourceID::ConnectionTimeout, b"");
                _ = close_sender.send(msg).await;
            });
            let timer_setter = timer.setter();
            monoio::spawn(async move {
                timer.await;
            });

            let timer_setter1 = timer_setter.clone();
            let task1 = async {
                loop {
                    match ReqwestMsgIOUtil::recv_msgs(&mut recv_stream).await {
                        Ok(msg) => {
                            let new_timeout = Instant::now() + idle_timeout;
                            timer_setter1.set(new_timeout).await;
                            if msg.resource_id() == ReqwestResourceID::Ping {
                                let msg = ReqwestMsg::with_resource_id_payload(
                                    ReqwestResourceID::Pong,
                                    b"",
                                );
                                _ = send_sender0.send(msg).await;
                                continue;
                            }
                            if let Err(_e) = recv_sender.send(msg).await {
                                error!("send msg error: channel closed.");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error: {}", e.to_string());
                            drop(recv_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            let timer_setter2 = timer_setter;
            let task2 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            timer_setter2.set(Instant::now() + idle_timeout).await;
                            if let Err(e) = ReqwestMsgIOUtil::send_msgs(msg, &mut send_stream).await
                            {
                                error!("send msg error: {}", e.to_string());
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            _ = send_stream.shutdown().await;
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn io_channels(
        &mut self,
    ) -> (mpsc::bounded::Tx<ReqwestMsg>, mpsc::bounded::Rx<ReqwestMsg>) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

mod tests {
    #[monoio::test(enable_timer = true)]
    async fn test() {
        use crate::net::SharedTimer;
        use monoio::time::Instant;
        use std::time::Duration;
        let t = Instant::now();
        let timer = SharedTimer::new(Duration::from_secs(1), async move {
            println!("timeout: {:?}", t.elapsed());
        });
        let timer_setter = timer.setter();
        monoio::spawn(async {
            timer.await;
            println!("timeout done");
        });
        let setter = timer_setter.clone();
        monoio::spawn(async move {
            timer_setter
                .set(Instant::now() + Duration::from_secs(3))
                .await;
            println!("set done");
        });

        monoio::spawn(async move {
            setter.set(Instant::now() + Duration::from_secs(4)).await;
            println!("set done");
        });
        monoio::time::sleep(Duration::from_secs(5)).await;
    }
}
