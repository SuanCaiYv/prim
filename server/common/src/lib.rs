pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;
pub mod cache;
pub type Result<T> = anyhow::Result<T>;

#[cfg(test)]
mod tests {
    #![warn(unused_extern_crates)]

    use crate::joy;

    #[test]
    fn it_works() {
        println!("{}", joy::banner());
    }
}
