pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;

#[cfg(test)]
mod tests {
    #![warn(unused_extern_crates)]
    #[test]
    fn it_works() {
        crate::net::server::ServerConfigBuilder::default();
    }
}
