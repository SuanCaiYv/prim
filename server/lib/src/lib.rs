pub mod cache;
pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;

pub type Result<T> = anyhow::Result<T>;
pub const MESSAGE_NODE_ID_BEGINNING: u32 = 1;
pub const SCHEDULER_NODE_ID_BEGINNING: u32 = 1 << 18 + 1;
pub const SEQNUM_NODE_ID_BEGINNING: u32 = 1 << 19 + 1;
pub const MSGPROCESSOR_ID_BEGINNING: u32 = 1 << 20 + 1;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
    }
}
