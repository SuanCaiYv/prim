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
        sync::Arc,
        time::{Duration, Instant}, cell::UnsafeCell,
    };

    use futures::Future;
    use tokio::time::Sleep;

    use crate::joy;

    struct Timer {
        timer: Arc<UnsafeCell<Pin<Box<Sleep>>>>,
    }

    impl Timer {
        fn new(callback: impl Future<Output = ()> + Send + 'static) -> Self {
            let timer = tokio::time::sleep(Duration::from_millis(3000));
            Box::pin(timer).as_mut().poll(cx)
            let a = Box::pin(timer);
            let timer = Arc::new(UnsafeCell::new(a));
            let timer1 = timer.clone();
            tokio::spawn(async move {
                timer1.get_mut().as_mut().await;
            });
            Self {
                timer: timer,
            }
        }

        fn reset_timer(&self, timeout: Instant) {
            self.timer.get_mut();
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
        timer.reset_timer(Instant::now() + Duration::from_millis(2000));
        tokio::time::sleep(Duration::from_millis(10000)).await;
    }
}
