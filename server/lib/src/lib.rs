pub mod cache;
pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;

pub type Result<T> = anyhow::Result<T>;
pub const MESSAGE_NODE_ID_BEGINNING: u32 = 1;
pub const SCHEDULER_NODE_ID_BEGINNING: u32 = 1 << 18 + 1;

pub fn from_std_res<T, E: std::fmt::Debug>(res: std::result::Result<T, E>) -> self::Result<T> {
    match res {
        Ok(r) => Ok(r),
        Err(e) => {
            let err = anyhow::anyhow!("{:?}", e);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };

    use futures::Future;
    use tokio::time::{Instant, Sleep};

    use crate::joy;

    struct TimerSetter {
        sender: tokio::sync::mpsc::Sender<Instant>,
    }

    impl TimerSetter {
        fn new(sender: tokio::sync::mpsc::Sender<Instant>) -> Self {
            Self { sender }
        }

        async fn set(&self, timeout: Instant) {
            _ = self.sender.send(timeout).await;
        }
    }

    struct Timer {
        timer: Pin<Box<Sleep>>,
        task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        sender: tokio::sync::mpsc::Sender<Instant>,
        receiver: tokio::sync::mpsc::Receiver<Instant>,
    }

    impl Timer {
        fn new(callback: impl Future<Output = ()> + Send + 'static) -> Self {
            let timer = tokio::time::sleep(Duration::from_millis(3000));
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            Self {
                timer: Box::pin(timer),
                task: Box::pin(callback),
                sender,
                receiver,
            }
        }

        fn setter(&self) -> TimerSetter {
            TimerSetter::new(self.sender.clone())
        }
    }

    impl Unpin for Timer {}

    impl Future for Timer {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
            match self.receiver.poll_recv(cx) {
                Poll::Pending => {
                    match self.timer.as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            match self.task.as_mut().poll(cx) {
                                Poll::Ready(_) => {
                                    Poll::Ready(())
                                }
                                Poll::Pending => {
                                    Poll::Pending
                                }
                            }
                        }
                        Poll::Pending => {
                            Poll::Pending
                        }
                    }
                }
                Poll::Ready(Some(timeout)) => {
                    self.timer.as_mut().reset(timeout);
                    match self.timer.as_mut().poll(cx) {
                        Poll::Ready(_) => {
                            match self.task.as_mut().poll(cx) {
                                Poll::Ready(_) => {
                                    Poll::Ready(())
                                }
                                Poll::Pending => {
                                    Poll::Pending
                                }
                            }
                        }
                        Poll::Pending => {
                            Poll::Pending
                        }
                    }
                }
                Poll::Ready(None) => {
                    Poll::Ready(())
                }
            }
        }
    }

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
        let v: u64 = 1 << 36;
        println!("{}", v);
    }

    #[tokio::test]
    async fn test() {
        let t = Instant::now();
        let timer = Timer::new(async move {
            println!("{:?}", t.elapsed());
        });
        let sender = timer.setter();
        tokio::spawn(async move {
            timer.await;
        });
        sender.set(Instant::now() + Duration::from_millis(1000)).await;
        tokio::time::sleep(Duration::from_millis(10000)).await;
    }
}
