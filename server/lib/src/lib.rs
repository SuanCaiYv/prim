pub mod entity;
pub mod error;
pub mod net;
pub mod util;
pub mod joy;
pub mod cache;

pub type Result<T> = anyhow::Result<T>;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        println!("{}", joy::banner())
    }
}
