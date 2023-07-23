pub mod net;

#[cfg(test)]
mod tests {

    use std::{
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::Duration,
    };

    use ahash::AHashMap;
    use dashmap::DashMap;
    use futures::Future;
    use lib::joy;
    use tokio::time::{Instant, Sleep};

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

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
        let v: u64 = 1 << 36;
        println!("{}", v);
        let m1 = Arc::new(DashMap::new());
        let m2 = Arc::new(Mutex::new(AHashMap::new()));
        let map1 = m1.clone();
        let map2 = m2.clone();
        std::thread::spawn(move || {
            let t = Instant::now();
            for i in 10000..20000 {
                map1.insert(i, i);
            }
            println!("m1 {:?}", t.elapsed());
            let t = Instant::now();
            for i in 10000..20000 {
                let mut map2 = map2.lock().unwrap();
                map2.insert(i, i);
            }
            println!("m2 {:?}", t.elapsed());
        });
        let map1 = m1.clone();
        let map2 = m2.clone();
        std::thread::spawn(move || {
            let t = Instant::now();
            for i in 20000..30000 {
                map1.insert(i, i);
            }
            println!("m1 {:?}", t.elapsed());
            let t = Instant::now();
            for i in 20000..30000 {
                let mut map2 = map2.lock().unwrap();
                map2.insert(i, i);
            }
            println!("m2 {:?}", t.elapsed());
        });
        let map1 = m1.clone();
        let map2 = m2.clone();
        std::thread::spawn(move || {
            let t = Instant::now();
            for i in 30000..40000 {
                map1.insert(i, i);
            }
            println!("m1 {:?}", t.elapsed());
            let t = Instant::now();
            for i in 30000..40000 {
                let mut map2 = map2.lock().unwrap();
                map2.insert(i, i);
            }
            println!("m2 {:?}", t.elapsed());
        });
        let t = Instant::now();
        for i in 0..10000 {
            m1.insert(i, i);
        }
        println!("m1 {:?}", t.elapsed());
        let t = Instant::now();
        for i in 0..10000 {
            let mut m2 = m2.lock().unwrap();
            m2.insert(i, i);
        }
        println!("m2 {:?}", t.elapsed());
    }

    #[tokio::test]
    async fn test() {
        println!("{}", chrono::Local::now().format("%Y-%m-%d-%H-%M-%S-%3f"));
        let t = Instant::now();
        let timer = Timer::new(async move {
            println!("{:?}", t.elapsed());
        });
        let sender = timer.setter();
        tokio::spawn(async move {
            timer.await;
        });
        sender
            .set(Instant::now() + Duration::from_millis(1000))
            .await;
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}
