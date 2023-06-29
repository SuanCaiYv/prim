use std::{
    any::type_name,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use ahash::AHashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use futures::Future;
use tokio::{
    sync::mpsc,
    time::{Instant, Sleep},
};

pub mod client;
pub mod server;

use crate::{
    entity::{Msg, ReqwestMsg, EXTENSION_THRESHOLD, PAYLOAD_THRESHOLD, ReqwestResourceID},
    Result,
};

pub const BODY_SIZE: usize = EXTENSION_THRESHOLD + PAYLOAD_THRESHOLD;
pub const ALPN_PRIM: &[&[u8]] = &[b"prim"];

pub type HandlerList = Arc<Vec<Box<dyn Handler>>>;
pub type InnerStates = AHashMap<String, InnerStatesValue>;

pub struct GenericParameterMap(pub AHashMap<&'static str, Box<dyn GenericParameter>>);

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

pub trait GenericParameter: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl GenericParameterMap {
    pub fn get_parameter<T: GenericParameter + 'static>(&self) -> Result<&T> {
        match self.0.get(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_any().downcast_ref::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter: {} not found", type_name::<T>())),
        }
    }

    pub fn get_parameter_mut<T: GenericParameter + 'static>(&mut self) -> Result<&mut T> {
        match self.0.get_mut(std::any::type_name::<T>()) {
            Some(parameter) => match parameter.as_mut_any().downcast_mut::<T>() {
                Some(parameter) => Ok(parameter),
                None => Err(anyhow!("parameter type mismatch")),
            },
            None => Err(anyhow!("parameter not found")),
        }
    }

    pub fn put_parameter<T: GenericParameter + 'static>(&mut self, parameter: T) {
        self.0
            .insert(std::any::type_name::<T>(), Box::new(parameter));
    }
}

pub enum InnerStatesValue {
    #[allow(unused)]
    Str(String),
    #[allow(unused)]
    Num(u64),
    #[allow(unused)]
    Bool(bool),
    #[allow(unused)]
    GenericParameterMap(GenericParameterMap),
}

impl InnerStatesValue {
    pub fn is_bool(&self) -> bool {
        matches!(*self, InnerStatesValue::Bool(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            InnerStatesValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_bool(&mut self) -> Option<&mut bool> {
        match *self {
            InnerStatesValue::Bool(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_num(&self) -> bool {
        matches!(*self, InnerStatesValue::Num(_))
    }

    pub fn as_num(&self) -> Option<u64> {
        match *self {
            InnerStatesValue::Num(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_num(&mut self) -> Option<&mut u64> {
        match *self {
            InnerStatesValue::Num(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_str(&self) -> bool {
        matches!(*self, InnerStatesValue::Str(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match *self {
            InnerStatesValue::Str(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_str(&mut self) -> Option<&mut String> {
        match *self {
            InnerStatesValue::Str(ref mut value) => Some(value),
            _ => None,
        }
    }

    pub fn is_generic_parameter_map(&self) -> bool {
        matches!(*self, InnerStatesValue::GenericParameterMap(_))
    }

    pub fn as_generic_parameter_map(&self) -> Option<&GenericParameterMap> {
        match *self {
            InnerStatesValue::GenericParameterMap(ref value) => Some(value),
            _ => None,
        }
    }

    pub fn as_mut_generic_parameter_map(&mut self) -> Option<&mut GenericParameterMap> {
        match *self {
            InnerStatesValue::GenericParameterMap(ref mut value) => Some(value),
            _ => None,
        }
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
