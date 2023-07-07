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
use async_recursion::async_recursion;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use futures::{future::BoxFuture, pin_mut, select, Future, FutureExt};
use lib::{
    entity::{
        Head, Msg, ReqwestMsg, ReqwestResourceID, Type, EXTENSION_THRESHOLD, HEAD_LEN,
        PAYLOAD_THRESHOLD,
    },
    error::CrashError,
    net::{GenericParameter, InnerStates},
    Result,
};
use quinn::{ReadExactError, RecvStream, SendStream};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::mpsc,
    time::{Instant, Sleep},
};
use tokio_rustls::{client as tls_client, server as tls_server};
use tracing::{debug, error, info, warn};

use self::server::ReqwestCaller;

#[cfg(not(feature = "no-check"))]
use lib::entity::msg::MSG_DELIMITER;

pub mod client;
pub mod server;

/// the direction is relative to the stream task.
///
/// why tokio? cause this direction's model is multi-sender and single-receiver
///
/// why async-channel? cause this direction's model is single-sender multi-receiver
pub type MsgMpmcReceiver = async_channel::Receiver<Arc<Msg>>;
pub type MsgMpmcSender = async_channel::Sender<Arc<Msg>>;
pub type MsgMpscSender = mpsc::Sender<Arc<Msg>>;
pub type MsgMpscReceiver = mpsc::Receiver<Arc<Msg>>;

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;

pub type ReqwestHandlerMap = Arc<AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>>>;
pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;
pub type ReqwestHandlerGenerator =
    Box<dyn Fn() -> Box<dyn NewReqwestConnectionHandler> + Send + Sync + 'static>;
pub(self) type ReqwestHandlerGenerator0 =
    Box<dyn Fn() -> Box<dyn NewReqwestConnectionHandler0> + Send + Sync + 'static>;

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// the [`msg`] can be modified before clone() has been called.
    /// so each handler modifying [`msg`] should be put on the top of the handler list.
    async fn run(
        &self,
        msg: &mut Arc<Msg>,
        // this one contains some states corresponding to the quic stream.
        states: &mut InnerStates,
    ) -> Result<Msg>;
}

#[async_trait]
pub trait ReqwestHandler: Send + Sync + 'static {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg>;
}

#[async_trait]
pub trait NewReqwestConnectionHandler: Send + Sync + 'static {
    async fn handle(
        &mut self,
        msg_operators: (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>),
    ) -> Result<()>;

    fn set_reqwest_caller(&mut self, reqwest_caller: ReqwestCaller);
}

#[async_trait]
pub(self) trait NewReqwestConnectionHandler0: Send + Sync + 'static {
    async fn handle(
        &mut self,
        msg_streams: (SendStream, RecvStream),
        client_caller: Option<Arc<ReqwestOperatorManager>>,
    ) -> Result<Option<ReqwestOperator>>;
}

#[derive(Clone)]
pub struct TimerSetter {
    sender: mpsc::Sender<Instant>,
}

impl TimerSetter {
    pub fn new(sender: mpsc::Sender<Instant>) -> Self {
        Self { sender }
    }

    pub async fn set(&self, new_timeout: Instant) {
        _ = self.sender.send(new_timeout).await;
    }
}

pub struct SharedTimer {
    timer: Pin<Box<Sleep>>,
    task: BoxFuture<'static, ()>,
    set_sender: mpsc::Sender<Instant>,
    set_receiver: mpsc::Receiver<Instant>,
}

impl SharedTimer {
    pub fn new(
        default_timeout: Duration,
        callback: impl Future<Output = ()> + Send + 'static,
    ) -> Self {
        let timer = tokio::time::sleep(default_timeout);
        let (set_sender, set_receiver) = mpsc::channel(1);
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

impl Unpin for SharedTimer {}

impl Future for SharedTimer {
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
    pub(crate) mpsc::Sender<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>,
);

pub struct Reqwest {
    req_id: u64,
    sender_task_done: bool,
    req: Option<ReqwestMsg>,
    operator_sender:
        Option<mpsc::Sender<(ReqwestMsg, Option<(u64, Arc<ResponsePlaceholder>, Waker)>)>>,
    sender_task: Option<BoxFuture<'static, Result<()>>>,
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
                        std::task::Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
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
                        if let Err(e) = operator_sender.send((req, Some((req_id, tx, waker)))).await
                        {
                            error!("send req error: {}", e.to_string());
                            return Err(anyhow!(e.to_string()));
                        }
                        Ok(())
                    };
                    let task: BoxFuture<Result<()>> = Box::pin(task);
                    self.sender_task = Some(task);
                    match self.sender_task.as_mut().unwrap().as_mut().poll(cx) {
                        std::task::Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    };
                }
            };
        }
        match self.resp_receiver.get() {
            Some(resp) => {
                self.load_counter.fetch_sub(1, Ordering::SeqCst);
                std::task::Poll::Ready(resp)
            }
            None => std::task::Poll::Pending,
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
            let load_val = load.load(Ordering::SeqCst);
            if load_val < min_load {
                min_load = load_val;
                min_index = i;
            }
        }
        (unsafe { &*self.load_list.get() })[min_index].fetch_add(1, Ordering::SeqCst);
        let req_id = self.req_id.fetch_add(1, Ordering::SeqCst);
        let operator = &(unsafe { &*self.operator_list.get() })[min_index];
        let req_sender = operator.1.clone();
        let resp_receiver = Arc::new(ResponsePlaceholder::new());
        req.set_req_id(req_id | self.target_mask);
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

impl GenericParameter for ReqwestOperatorManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[inline(always)]
#[cfg(not(feature = "no-check"))]
pub(self) fn pre_check(msg: &[u8]) -> usize {
    if msg.len() < 4 {
        return msg.len();
    }
    // by test, directly iter is fast enough to work.
    let mut i = 0;
    while i < msg.len() - 3 {
        if msg[i] == MSG_DELIMITER[0] {
            if msg[i + 1] == MSG_DELIMITER[1] {
                if msg[i + 2] == MSG_DELIMITER[2] {
                    if msg[i + 3] == MSG_DELIMITER[3] {
                        return i;
                    } else {
                        i += 4;
                    }
                } else {
                    i += 3;
                }
            } else {
                i += 2;
            }
        } else {
            i += 1;
        }
    }
    msg.len()
}

#[derive(Clone)]
pub enum MsgSender {
    Client(MsgMpmcSender),
    Server(MsgMpscSender),
}

impl MsgSender {
    pub async fn send(&self, msg: Arc<Msg>) -> Result<()> {
        match self {
            MsgSender::Client(sender) => {
                sender.send(msg).await?;
            }
            MsgSender::Server(sender) => {
                sender.send(msg).await?;
            }
        }
        Ok(())
    }

    pub fn close(self) {
        match self {
            MsgSender::Client(sender) => {
                sender.close();
            }
            MsgSender::Server(sender) => {
                drop(sender);
            }
        }
    }
}

/// read bytes from stream, if external_source is not None, read from external_source first,
/// and return the rest of external_source if remained.
#[inline(always)]
pub(self) async fn read_buffer<'a>(
    recv_stream: &mut RecvStream,
    external_source: Option<&'a [u8]>,
    buffer: &mut [u8],
) -> Result<Option<&'a [u8]>> {
    match external_source {
        Some(external_source) => {
            if external_source.len() < buffer.len() {
                buffer[0..external_source.len()].copy_from_slice(external_source);
                match recv_stream
                    .read_exact(&mut buffer[external_source.len()..])
                    .await
                {
                    Ok(_) => Ok(None),
                    Err(e) => match e {
                        ReadExactError::FinishedEarly => {
                            info!("stream finished.");
                            Err(anyhow!(CrashError::ShouldCrash(
                                "stream finished.".to_string()
                            )))
                        }
                        ReadExactError::ReadError(e) => {
                            debug!("read stream error: {:?}", e);
                            Err(anyhow!(CrashError::ShouldCrash(
                                "read stream error.".to_string()
                            )))
                        }
                    },
                }
            } else {
                buffer.copy_from_slice(&external_source[0..buffer.len()]);
                Ok(Some(&external_source[buffer.len()..]))
            }
        }
        None => match recv_stream.read_exact(buffer).await {
            Ok(_) => Ok(None),
            Err(e) => match e {
                ReadExactError::FinishedEarly => {
                    info!("stream finished.");
                    Err(anyhow!(CrashError::ShouldCrash(
                        "stream finished.".to_string()
                    )))
                }
                ReadExactError::ReadError(e) => {
                    debug!("read stream error: {:?}", e);
                    Err(anyhow!(CrashError::ShouldCrash(
                        "read stream error.".to_string()
                    )))
                }
            },
        },
    }
}

pub(self) struct MsgIOUtil;

impl MsgIOUtil {
    /// the only error returned should cause the stream crashed.
    ///
    /// the purpose using [`std::sync::Arc`] is to reduce unnecessary memory copy.
    #[async_recursion]
    pub(self) async fn recv_msg<'a: 'async_recursion>(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut RecvStream,
        mut external_source: Option<&'a [u8]>,
    ) -> Result<Arc<Msg>> {
        #[cfg(not(feature = "no-check"))]
        {
            let mut from = 0;
            let mut delimiter_buf = [0u8; 4];
            let mut loss = 0;
            loop {
                match read_buffer(recv_stream, external_source, &mut delimiter_buf[from..]).await {
                    Ok(external_source0) => {
                        external_source = external_source0;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                if delimiter_buf[0] != MSG_DELIMITER[0] {
                    loss += 1;
                    debug!("invalid message detected[1].");
                    delimiter_buf[0] = delimiter_buf[1];
                    delimiter_buf[1] = delimiter_buf[2];
                    delimiter_buf[2] = delimiter_buf[3];
                    from = 3;
                    continue;
                } else if delimiter_buf[1] != MSG_DELIMITER[1] {
                    loss += 2;
                    debug!("invalid message detected[2].");
                    delimiter_buf[0] = delimiter_buf[2];
                    delimiter_buf[1] = delimiter_buf[3];
                    from = 2;
                    continue;
                } else if delimiter_buf[2] != MSG_DELIMITER[2] {
                    loss += 3;
                    debug!("invalid message detected[3].");
                    delimiter_buf[0] = delimiter_buf[3];
                    from = 1;
                    continue;
                } else if delimiter_buf[3] != MSG_DELIMITER[3] {
                    loss += 4;
                    debug!("invalid message detected[4].");
                    from = 0;
                } else {
                    break;
                }
            }
            if loss != 0 {
                error!(
                    "{} message loss {} bytes detected.",
                    recv_stream.id().0,
                    loss
                );
            }
        }
        match read_buffer(recv_stream, external_source, &mut buffer[..]).await {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(not(feature = "no-check"))]
        {
            let index = pre_check(&buffer[..]);
            if index != buffer.len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&buffer[index..]);
                let res = MsgIOUtil::recv_msg(buffer, recv_stream, Some(&external)).await;
                return res;
            }
        }
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut head = Head::from(&buffer[..]);
        let mut msg = Msg::pre_alloc(&mut head);
        match read_buffer(
            recv_stream,
            external_source,
            &mut msg.as_mut_slice()[HEAD_LEN..],
        )
        .await
        {
            Ok(_external_source) => {
                #[cfg(not(feature = "no-check"))]
                {
                    external_source = _external_source;
                }
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(not(feature = "no-check"))]
        {
            let index = pre_check(msg.as_slice());
            if index != msg.as_slice().len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&msg.as_slice()[index..]);
                let res = MsgIOUtil::recv_msg(buffer, recv_stream, Some(&external)).await;
                return res;
            }
        }
        Ok(Arc::new(msg))
    }

    #[inline(always)]
    pub(self) async fn recv_msgs(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut ReadHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<Arc<Msg>> {
        match recv_stream.read_exact(&mut buffer[..]).await {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let mut head = Head::from(&buffer[..]);
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(&mut head);
        match recv_stream
            .read_exact(&mut (msg.as_mut_slice()[HEAD_LEN..]))
            .await
        {
            Ok(_) => {}
            Err(_) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        debug!("read msg: {}", msg);
        Ok(Arc::new(msg))
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn recv_msgc(
        buffer: &mut Box<[u8; HEAD_LEN]>,
        recv_stream: &mut ReadHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<Arc<Msg>> {
        match recv_stream.read_exact(&mut buffer[..]).await {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        let mut head = Head::from(&buffer[..]);
        if (Head::extension_length(&buffer[..]) + Head::payload_length(&buffer[..])) > BODY_SIZE {
            return Err(anyhow!(CrashError::ShouldCrash(
                "message size too large.".to_string()
            )));
        }
        let mut msg = Msg::pre_alloc(&mut head);
        match recv_stream
            .read_exact(&mut (msg.as_mut_slice()[HEAD_LEN..]))
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        }
        debug!("read msg: {}", msg);
        Ok(Arc::new(msg))
    }

    /// the only error returned should cause the stream crashed.
    /// and this method will automatically finish the stream.
    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msg(msg: Arc<Msg>, send_stream: &mut SendStream) -> Result<()> {
        #[cfg(not(feature = "no-check"))]
        if pre_check(msg.as_slice()) != msg.as_slice().len() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "invalid message.".to_string()
            )));
        }
        #[cfg(not(feature = "no-check"))]
        if let Err(e) = send_stream.write_all(&MSG_DELIMITER).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.finish().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msgs(
        msg: Arc<Msg>,
        send_stream: &mut WriteHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }

    #[allow(unused)]
    #[inline]
    pub(self) async fn send_msgc(
        msg: Arc<Msg>,
        send_stream: &mut WriteHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {}", msg);
        Ok(())
    }
}

pub struct MsgIOWrapper {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
}

impl MsgIOWrapper {
    pub(self) fn new(mut send_stream: SendStream, mut recv_stream: RecvStream) -> Self {
        // actually channel buffer size set to 1 is more intuitive.
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            mpsc::channel(16384);
        let (recv_sender, recv_receiver): (MsgMpscSender, MsgMpscReceiver) = mpsc::channel(16284);
        tokio::spawn(async move {
            let task1 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) = MsgIOUtil::send_msg(msg, &mut send_stream).await {
                                error!("send msg error: {:?}", e);
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                let mut buffer = Box::new([0u8; HEAD_LEN]);
                loop {
                    match MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream, None).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error {}.", e);
                            // try to notice receiver to stop.
                            drop(recv_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                // why we choose to use futures::select!{} but tokio::select!{}?
                // the reason is that tokio::select!{} has bug in high concurrent network request.
                // but with futures::select!{}, some code may run slower caused by mutable reference required by futures::select!{}.
                // (to locate this bug takes me 4 days 😢
                futures::select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    complete => {
                        break;
                    }
                }
            }
        });
        // tokio::spawn(async move {
        //     let mut buffer = Box::new([0u8; HEAD_LEN]);
        //     loop {
        //         match MsgIOUtil::recv_msg(&mut buffer, &mut recv_stream).await {
        //             Ok(msg) => {
        //                 if let Err(e) = recv_sender.send(msg).await {
        //                     error!("send msg error: {:?}", e);
        //                     break;
        //                 }
        //             }
        //             Err(_) => {
        //                 break;
        //             }
        //         }
        //     }
        // });
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub struct MsgIOWrapperTcpS {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
}

impl MsgIOWrapperTcpS {
    pub(self) fn new(stream: tls_server::TlsStream<TcpStream>, idle_timeout: Duration) -> Self {
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            mpsc::channel(16384);
        let (recv_sender, recv_receiver) = mpsc::channel(16384);
        let (mut recv_stream, mut send_stream) = split(stream);
        let close_sender = send_sender.clone();
        let send_sender0 = send_sender.clone();
        tokio::spawn(async move {
            let mut buffer = Box::new([0u8; HEAD_LEN]);
            let timer = SharedTimer::new(idle_timeout, async move {
                let mut msg = Msg::raw(0, 0, 0, &[]);
                msg.set_type(Type::Close);
                _ = close_sender.send(Arc::new(msg)).await;
            });
            let timer_setter = timer.setter();
            tokio::spawn(async move {
                timer.await;
            });

            let timer_setter1 = timer_setter.clone();
            let task1 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if msg.typ() == Type::Close {
                                _ = send_stream.shutdown().await;
                            }
                            timer_setter1
                                .set(tokio::time::Instant::now() + idle_timeout)
                                .await;
                            if let Err(e) =
                                MsgIOUtil::send_msgs(msg.clone(), &mut send_stream).await
                            {
                                error!("send msg error: {:?}", e);
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let timer_setter2 = timer_setter;
            let task2 = async {
                loop {
                    match MsgIOUtil::recv_msgs(&mut buffer, &mut recv_stream).await {
                        Ok(msg) => {
                            timer_setter2
                                .set(tokio::time::Instant::now() + idle_timeout)
                                .await;
                            if msg.typ() == Type::Ping {
                                let msg = Arc::new(Msg::pong(0, 0, 0));
                                _ = send_sender0.send(msg).await;
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error {}.", e);
                            drop(recv_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2);

            loop {
                futures::select! {
                    _ = task1 => {
                    },
                    _ = task2 => {
                    }
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

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub(self) struct MsgIOWrapperTcpC {
    pub(self) send_channel: Option<MsgMpscSender>,
    pub(self) recv_channel: Option<MsgMpscReceiver>,
}

impl MsgIOWrapperTcpC {
    pub(self) fn new(
        stream: tls_client::TlsStream<TcpStream>,
        keep_alive_interval: Duration,
    ) -> Self {
        let (send_sender, mut send_receiver): (MsgMpscSender, MsgMpscReceiver) =
            mpsc::channel(16384);
        let (recv_sender, recv_receiver) = mpsc::channel(16384);
        let (mut recv_stream, mut send_stream) = split(stream);
        let tick_sender = send_sender.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(keep_alive_interval);

            let task1 = async move {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) =
                                MsgIOUtil::send_msgc(msg.clone(), &mut send_stream).await
                            {
                                error!("send msg error: {:?}", e);
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async move {
                let mut buffer = Box::new([0u8; HEAD_LEN]);
                loop {
                    match MsgIOUtil::recv_msgc(&mut buffer, &mut recv_stream).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error {}.", e);
                            drop(recv_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task3 = async move {
                loop {
                    ticker.tick().await;
                    let msg = Arc::new(Msg::ping(0, 0, 0));
                    if let Err(e) = tick_sender.send(msg).await {
                        error!("send msg error: {:?}", e);
                        break;
                    }
                }
            }
            .fuse();

            pin_mut!(task1, task2, task3);

            loop {
                select! {
                    _ = task1 => {},
                    _ = task2 => {},
                    _ = task3 => {},
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

    pub fn channels(&mut self) -> (MsgMpscSender, MsgMpscReceiver) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub struct ReqwestMsgIOUtil {}

impl ReqwestMsgIOUtil {
    #[inline(always)]
    pub(self) async fn send_msg(msg: &ReqwestMsg, send_stream: &mut SendStream) -> Result<()> {
        #[cfg(not(feature = "no-check"))]
        if pre_check(msg.as_slice()) != msg.as_slice().len() {
            return Err(anyhow!(CrashError::ShouldCrash(
                "invalid message.".to_string()
            )));
        }
        #[cfg(not(feature = "no-check"))]
        if let Err(e) = send_stream.write_all(&MSG_DELIMITER).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        Ok(())
    }

    #[async_recursion]
    pub(self) async fn recv_msg<'a: 'async_recursion, 'b: 'async_recursion>(
        recv_stream: &mut RecvStream,
        #[allow(unused_variables)] mut external_source: Option<&'b [u8]>,
    ) -> Result<ReqwestMsg> {
        #[cfg(not(feature = "no-check"))]
        {
            let mut from = 0;
            let mut delimiter_buf = [0u8; 4];
            let mut loss = 0;
            loop {
                match read_buffer(recv_stream, external_source, &mut delimiter_buf[from..]).await {
                    Ok(external_source0) => {
                        external_source = external_source0;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                if delimiter_buf[0] != MSG_DELIMITER[0] {
                    loss += 1;
                    debug!("invalid message detected[1].");
                    delimiter_buf[0] = delimiter_buf[1];
                    delimiter_buf[1] = delimiter_buf[2];
                    delimiter_buf[2] = delimiter_buf[3];
                    from = 3;
                    continue;
                } else if delimiter_buf[1] != MSG_DELIMITER[1] {
                    loss += 2;
                    debug!("invalid message detected[2].");
                    delimiter_buf[0] = delimiter_buf[2];
                    delimiter_buf[1] = delimiter_buf[3];
                    from = 2;
                    continue;
                } else if delimiter_buf[2] != MSG_DELIMITER[2] {
                    loss += 3;
                    debug!("invalid message detected[3].");
                    delimiter_buf[0] = delimiter_buf[3];
                    from = 1;
                    continue;
                } else if delimiter_buf[3] != MSG_DELIMITER[3] {
                    loss += 4;
                    debug!("invalid message detected[4].");
                    from = 0;
                } else {
                    break;
                }
            }
            if loss != 0 {
                error!(
                    "{} message loss {} bytes detected.",
                    recv_stream.id().0,
                    loss
                );
            }
        }
        let mut len_buf: [u8; 2] = [0u8; 2];
        match read_buffer(recv_stream, external_source, &mut len_buf).await {
            Ok(external_source0) => {
                external_source = external_source0;
            }
            Err(e) => {
                return Err(e);
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = ReqwestMsg::pre_alloc(len);
        match read_buffer(recv_stream, external_source, &mut msg.body_mut()).await {
            Ok(_external_source) => {
                #[cfg(not(feature = "no-check"))]
                {
                    external_source = _external_source;
                }
            }
            Err(e) => {
                return Err(e);
            }
        };
        #[cfg(not(feature = "no-check"))]
        {
            let index = pre_check(msg.as_slice());
            if index != msg.as_slice().len() {
                error!("invalid message detected.");
                let mut external;
                match external_source {
                    Some(external_source0) => {
                        external = external_source0.to_owned();
                    }
                    None => {
                        external = vec![];
                    }
                }
                external.extend_from_slice(&msg.as_slice()[index..]);
                let res = ReqwestMsgIOUtil::recv_msg(recv_stream, Some(&external)).await;
                return res;
            }
        }
        Ok(msg)
    }

    #[inline(always)]
    pub(self) async fn send_msgc(
        msg: &ReqwestMsg,
        send_stream: &mut WriteHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {:#?}", msg);
        Ok(())
    }

    #[inline(always)]
    pub(self) async fn send_msgs(
        msg: &ReqwestMsg,
        send_stream: &mut WriteHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<()> {
        if let Err(e) = send_stream.write_all(msg.as_slice()).await {
            _ = send_stream.shutdown().await;
            debug!("write stream error: {:?}", e);
            return Err(anyhow!(CrashError::ShouldCrash(
                "write stream error.".to_string()
            )));
        }
        debug!("write msg: {:#?}", msg);
        Ok(())
    }

    #[inline(always)]
    pub(self) async fn recv_msgc(
        recv_stream: &mut ReadHalf<tls_client::TlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let mut len_buf: [u8; 2] = [0u8; 2];
        match recv_stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) => {
                error!("read stream error: {:?}", e);
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = ReqwestMsg::pre_alloc(len);
        match recv_stream.read_exact(&mut msg.body_mut()).await {
            Ok(_) => {}
            Err(e) => {
                error!("read stream error: {:?}", e);
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        Ok(msg)
    }

    #[inline(always)]
    pub(self) async fn recv_msgs(
        recv_stream: &mut ReadHalf<tls_server::TlsStream<TcpStream>>,
    ) -> Result<ReqwestMsg> {
        let mut len_buf: [u8; 2] = [0u8; 2];
        match recv_stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) => {
                error!("read stream error: {:?}", e);
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        let len = BigEndian::read_u16(&len_buf[..]);
        let mut msg = ReqwestMsg::pre_alloc(len);
        match recv_stream.read_exact(&mut msg.body_mut()).await {
            Ok(_) => {}
            Err(e) => {
                error!("read stream error: {:?}", e);
                return Err(anyhow!(CrashError::ShouldCrash(
                    "read stream error.".to_string()
                )));
            }
        };
        Ok(msg)
    }
}

pub struct ReqwestMsgIOWrapper {
    pub(self) send_channel: Option<mpsc::Sender<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::Receiver<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapper {
    pub fn new(mut send_stream: SendStream, mut recv_stream: RecvStream) -> Self {
        let (send_sender, mut send_receiver): (
            mpsc::Sender<ReqwestMsg>,
            mpsc::Receiver<ReqwestMsg>,
        ) = mpsc::channel(16384);
        let (recv_sender, recv_receiver): (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) =
            mpsc::channel(16384);
        #[cfg(not(feature = "no-select"))]
        tokio::spawn(async move {
            let task1 = async {
                loop {
                    match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {}", e.to_string());
                                _ = recv_stream.stop(0u32.into());
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error: {}", e.to_string());
                            drop(recv_sender);
                            _ = recv_stream.stop(0u32.into());
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task2 = async {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) = ReqwestMsgIOUtil::send_msg(&msg, &mut send_stream).await
                            {
                                error!("send msg error: {}", e.to_string());
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            _ = send_stream.finish().await;
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
        #[cfg(feature = "no-select")]
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
                        _ = send_stream.finish().await;
                        break;
                    }
                }
            }
        });
        #[cfg(feature = "no-select")]
        tokio::spawn(async move {
            loop {
                match ReqwestMsgIOUtil::recv_msg(&mut recv_stream, None).await {
                    Ok(msg) => {
                        if let Err(e) = recv_sender.send(msg).await {
                            error!("send msg error: {}", e.to_string());
                            break;
                        }
                    }
                    Err(_) => {
                        _ = recv_stream.stop(0u32.into());
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

    pub fn channels(&mut self) -> (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub struct ReqwestMsgIOWrapperTcpC {
    pub(self) send_channel: Option<mpsc::Sender<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::Receiver<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapperTcpC {
    pub fn new(stream: tls_client::TlsStream<TcpStream>, keep_alive_interval: Duration) -> Self {
        let (send_sender, mut send_receiver): (
            mpsc::Sender<ReqwestMsg>,
            mpsc::Receiver<ReqwestMsg>,
        ) = mpsc::channel(16384);
        let (recv_sender, recv_receiver): (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) =
            mpsc::channel(16384);
        let tick_sender = send_sender.clone();
        tokio::spawn(async move {
            let (mut recv_stream, mut send_stream) = split(stream);
            let mut ticker = tokio::time::interval(keep_alive_interval);

            let task1 = async move {
                loop {
                    match send_receiver.recv().await {
                        Some(msg) => {
                            if let Err(e) =
                                ReqwestMsgIOUtil::send_msgc(&msg, &mut send_stream).await
                            {
                                error!("send msg error: {:?}", e);
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

            let task2 = async move {
                loop {
                    match ReqwestMsgIOUtil::recv_msgc(&mut recv_stream).await {
                        Ok(msg) => {
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {:?}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("recv msg error: {:?}", e);
                            drop(recv_sender);
                            break;
                        }
                    }
                }
            }
            .fuse();

            let task3 = async move {
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
        Self {
            send_channel: Some(send_sender),
            recv_channel: Some(recv_receiver),
        }
    }

    pub fn channels(&mut self) -> (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}

pub struct ReqwestMsgIOWrapperTcpS {
    pub(self) send_channel: Option<mpsc::Sender<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::Receiver<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapperTcpS {
    pub fn new(stream: tls_server::TlsStream<TcpStream>, idle_timeout: Duration) -> Self {
        let (send_sender, mut send_receiver): (
            mpsc::Sender<ReqwestMsg>,
            mpsc::Receiver<ReqwestMsg>,
        ) = mpsc::channel(16384);
        let (recv_sender, recv_receiver): (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) =
            mpsc::channel(16384);
        let send_sender0 = send_sender.clone();
        let close_sender = send_sender.clone();
        tokio::spawn(async move {
            let (mut recv_stream, mut send_stream) = split(stream);
            let timer = SharedTimer::new(idle_timeout, async move {
                let msg =
                    ReqwestMsg::with_resource_id_payload(ReqwestResourceID::ConnectionTimeout, b"");
                _ = close_sender.send(msg).await;
            });
            let timer_setter = timer.setter();
            tokio::spawn(async move {
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
                            error!("connection close: {}.", e);
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
                            if let Err(e) =
                                ReqwestMsgIOUtil::send_msgs(&msg, &mut send_stream).await
                            {
                                error!("send msg error: {}", e.to_string());
                                send_receiver.close();
                                break;
                            }
                        }
                        None => {
                            _ = send_stream.shutdown().await;
                            warn!("shutdown connection.");
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

    pub fn io_channels(&mut self) -> (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
}
