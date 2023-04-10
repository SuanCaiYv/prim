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

    use std::sync::Arc;

    use crate::joy;

    struct S {
        v1: i32,
    }

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
        let v: u64 = 1 << 36;
        println!("{}", v);
        let mut s = Arc::new(S { v1: 1 });
        let mut s1 = Arc::get_mut(&mut s).unwrap();
        for i in 0..5 {
            s1.v1 = i;
            s1 = f(s1);
            println!("{}", s1.v1);
        }
        println!("{}", s.v1);
    }

    fn f(s: &mut S) -> &mut S {
        s.v1 = s.v1 * 2;
        s
    }
}
