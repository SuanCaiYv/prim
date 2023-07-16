use std::{
    cell::UnsafeCell,
    pin::Pin,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    task::{Context, Poll, Waker},
};

use byteorder::{BigEndian, ByteOrder};
use dashmap::DashMap;
use futures::{future::BoxFuture, Future};
use lib::{entity::Msg, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tracing::error;

pub struct ResponsePlaceholder {
    value: UnsafeCell<Option<Result<()>>>,
}

impl ResponsePlaceholder {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    pub fn set(&self, new_value: Result<()>) {
        unsafe {
            (&mut (*self.value.get())).replace(new_value);
        }
    }

    pub fn get(&self) -> Option<Result<()>> {
        unsafe { (&mut (*self.value.get())).take() }
    }
}

unsafe impl Send for ResponsePlaceholder {}
unsafe impl Sync for ResponsePlaceholder {}

pub(super) struct MsgloggerClient {
    inner: mpsc::Sender<(u64, Arc<Msg>, Waker, Arc<ResponsePlaceholder>)>,
    id: AtomicU64,
}

impl MsgloggerClient {
    pub(super) async fn new(address: String) -> Result<Self> {
        let stream = tokio::net::UnixStream::connect(address).await?;
        let (mut reader, mut writer) = tokio::io::split(stream);
        let (tx, mut rx) = mpsc::channel::<(u64, Arc<Msg>, Waker, Arc<ResponsePlaceholder>)>(16384);
        let waker_map = Arc::new(DashMap::<u64, (Waker, Arc<ResponsePlaceholder>)>::new());

        let waker_map0 = waker_map.clone();
        tokio::spawn(async move {
            let mut id_buf = [0u8; 8];
            loop {
                match rx.recv().await {
                    Some(req) => {
                        BigEndian::write_u64(&mut id_buf, req.0);
                        if let Err(e) = writer.write_all(&id_buf).await {
                            error!("write id error: {:?}", e);
                            break;
                        }
                        if let Err(e) = writer.write_all(req.1.as_slice()).await {
                            error!("write msg error: {:?}", e);
                            break;
                        }
                        waker_map0.insert(req.0, (req.2, req.3));
                    }
                    None => {
                        break;
                    }
                }
            }
        });
        tokio::spawn(async move {
            let mut id_buf = [0u8; 8];
            loop {
                match reader.read_exact(&mut id_buf).await {
                    Ok(_) => {
                        let id = BigEndian::read_u64(&id_buf);
                        let waker = waker_map.remove(&id);
                        if let Some(waker) = waker {
                            waker.1.1.set(Ok(()));
                            waker.1.0.wake();
                        }
                    }
                    Err(e) => {
                        error!("read id error: {:?}", e);
                        break;
                    }
                }
            }
        });
        Ok(Self { inner: tx, id: AtomicU64::new(0) })
    }

    pub(super) fn call(&self, msg: Arc<Msg>) -> MsgloggerReqwest {
        let tx = self.inner.clone();
        MsgloggerReqwest::new(self.id.fetch_add(1, Ordering::Acquire), msg, tx)
    }
}

pub(super) struct MsgloggerReqwest {
    result: Arc<ResponsePlaceholder>,
    id: u64,
    msg: Arc<Msg>,
    sender: mpsc::Sender<(u64, Arc<Msg>, Waker, Arc<ResponsePlaceholder>)>,
    sender_task: Option<BoxFuture<'static, Result<()>>>,
    sender_task_done: bool,
}

impl Unpin for MsgloggerReqwest {}

impl Future for MsgloggerReqwest {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.sender_task_done {
            match self.sender_task.as_mut() {
                Some(task) => {
                    match task.as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    };
                }
                None => {
                    let id = self.id;
                    let msg = self.msg.clone();
                    let waker = cx.waker().clone();
                    let sender = self.sender.clone();
                    let result_placeholder = self.result.clone();
                    let task = async move {
                        sender.send((id, msg, waker, result_placeholder)).await?;
                        Ok(())
                    };
                    let task: BoxFuture<'static, Result<()>> = Box::pin(task);
                    self.sender_task.replace(task);
                    match self.sender_task.as_mut().unwrap().as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            self.sender_task_done = true;
                        }
                        Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    };
                }
            }
        }
        match self.result.get() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

impl MsgloggerReqwest {
    pub(super) fn new(
        id: u64,
        msg: Arc<Msg>,
        sender: mpsc::Sender<(u64, Arc<Msg>, Waker, Arc<ResponsePlaceholder>)>,
    ) -> Self {
        Self {
            result: Arc::new(ResponsePlaceholder::new()),
            id,
            msg,
            sender,
            sender_task: None,
            sender_task_done: false,
        }
    }
}
