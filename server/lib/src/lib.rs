pub mod cache;
pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;

pub type Result<T> = anyhow::Result<T>;
pub const MESSAGE_NODE_ID_BEGINNING: u32 = 1;
pub const SCHEDULER_NODE_ID_BEGINNING: u32 = 1 << 18 + 1;
pub const RECORDER_NODE_ID_BEGINNING: u32 = 1 << 18 + 1 << 16 + 1;

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
    use std::sync::Arc;

    use crate::joy;

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
        let v: u64 = 1 << 36;
        println!("{}", v);
        type t = Box<dyn Fn() -> Box<dyn Fn() -> i32 + Send + Sync + 'static> + Send + Sync + 'static>;
        let v: t = Box::new(|| Box::new(|| 1));
        let v1 = Arc::new(v);
        let v2 = v1.clone();
        let v3 = v1.clone();
        std::thread::spawn(move || {
            println!("{}", (v1)()());
        });
        std::thread::spawn(move || {
            println!("{}", (v2)()())
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
