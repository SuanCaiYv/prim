use std::{time::Duration, pin::Pin, task::{Context, Poll}, sync::Arc};

use anyhow::anyhow;
use ahash::AHashMap;
use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use futures::{pin_mut, FutureExt, Future};
use lib::{
    entity::{ReqwestMsg, ReqwestResourceID},
    error::CrashError,
    Result,
    net::InnerStates,
};
use monoio::{
    io::{
        AsyncReadRentExt, AsyncWriteRent, AsyncWriteRentExt, OwnedReadHalf, OwnedWriteHalf,
        Splitable,
    },
    net::TcpStream, time::{Sleep, Instant},
};
use monoio_rustls::server::TlsStream as STlsStream;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub mod server;

pub type ReqwestHandlerMap = Arc<AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>>>;

#[async_trait]
pub trait ReqwestHandler: 'static {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg>;
}

pub struct ReqwestMsgIOUtil {}

impl ReqwestMsgIOUtil {
    #[inline(always)]
    pub async fn send_msgs(
        msg: ReqwestMsg,
        stream: &mut OwnedWriteHalf<STlsStream<TcpStream>>,
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
            Ok(_) => Ok(ReqwestMsg(msg)),
        }
    }

    #[inline(always)]
    pub async fn recv_msgs(
        stream: &mut OwnedReadHalf<STlsStream<TcpStream>>,
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
    pub(self) send_channel: Option<mpsc::Sender<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::Receiver<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapper {
    pub fn new(stream: STlsStream<TcpStream>, idle_timeout: Duration) -> Self {
        let (send_sender, mut send_receiver): (
            mpsc::Sender<ReqwestMsg>,
            mpsc::Receiver<ReqwestMsg>,
        ) = mpsc::channel(16384);
        let (recv_sender, recv_receiver): (
            mpsc::Sender<ReqwestMsg>,
            mpsc::Receiver<ReqwestMsg>,
        ) = mpsc::channel(16384);
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
                            timer_setter1
                                .set(Instant::now() + idle_timeout)
                                .await;
                            if msg.resource_id() == ReqwestResourceID::Ping {
                                let msg = ReqwestMsg::with_resource_id_payload(
                                    ReqwestResourceID::Pong,
                                    b"",
                                );
                                _ = send_sender0.send(msg).await;
                                continue;
                            }
                            if let Err(e) = recv_sender.send(msg).await {
                                error!("send msg error: {}", e.to_string());
                                break;
                            }
                        }
                        Err(_) => {
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
                            timer_setter2
                                .set(Instant::now() + idle_timeout)
                                .await;
                            if let Err(e) = ReqwestMsgIOUtil::send_msgs(msg, &mut send_stream).await
                            {
                                error!("send msg error: {}", e.to_string());
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

    pub fn channels(&mut self) -> (mpsc::Sender<ReqwestMsg>, mpsc::Receiver<ReqwestMsg>) {
        let send = self.send_channel.take().unwrap();
        let recv = self.recv_channel.take().unwrap();
        (send, recv)
    }
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
    task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    set_sender: mpsc::Sender<Instant>,
    set_receiver: mpsc::Receiver<Instant>,
}

impl SharedTimer {
    pub fn new(
        default_timeout: Duration,
        callback: impl Future<Output = ()> + Send + 'static,
    ) -> Self {
        let timer = monoio::time::sleep(default_timeout);
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
        match self.set_receiver.poll_recv(cx) {
            Poll::Pending => match self.timer.as_mut().poll(cx) {
                Poll::Ready(_) => match self.task.as_mut().poll(cx) {
                    Poll::Ready(_) => Poll::Ready(()),
                    Poll::Pending => Poll::Pending,
                },
                Poll::Pending => Poll::Pending,
            },
            Poll::Ready(Some(timeout)) => {
                self.timer.as_mut().reset(timeout);
                match self.timer.as_mut().poll(cx) {
                    Poll::Ready(_) => match self.task.as_mut().poll(cx) {
                        Poll::Ready(_) => Poll::Ready(()),
                        Poll::Pending => Poll::Pending,
                    },
                    Poll::Pending => Poll::Pending,
                }
            }
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}
