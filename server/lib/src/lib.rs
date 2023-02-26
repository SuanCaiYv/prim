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
    use std::net::ToSocketAddrs;
    use super::*;

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
        let v: u64 = 1 << 36;
        println!("{}", v);
        let v: Vec<_> = "aaa.bbb:2323".to_socket_addrs().expect("parse failed").collect();
        let addr = v[0];
        println!("{}", addr.is_ipv4());
    }
}
