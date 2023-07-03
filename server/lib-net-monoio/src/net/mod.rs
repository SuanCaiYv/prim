use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
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
use monoio_rustls::server::TlsStream as STlsStream;
use tracing::{debug, error};

pub mod server;

pub type ReqwestHandlerMap = Arc<AHashMap<ReqwestResourceID, Box<dyn ReqwestHandler>>>;

#[async_trait(? Send)]
pub trait ReqwestHandler: 'static {
    async fn run(&self, req: &mut ReqwestMsg, states: &mut InnerStates) -> Result<ReqwestMsg>;
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
            Ok(_size) => Ok(ReqwestMsg(msg)),
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
    pub(self) send_channel: Option<mpsc::bounded::Tx<ReqwestMsg>>,
    pub(self) recv_channel: Option<mpsc::bounded::Rx<ReqwestMsg>>,
}

impl ReqwestMsgIOWrapper {
    pub fn new(stream: STlsStream<TcpStream>, idle_timeout: Duration) -> Self {
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
                            timer_setter2.set(Instant::now() + idle_timeout).await;
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

    pub fn channels(&mut self) -> (mpsc::bounded::Tx<ReqwestMsg>, mpsc::bounded::Rx<ReqwestMsg>) {
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
