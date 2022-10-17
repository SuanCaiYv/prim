pub mod entity;
pub mod error;
pub mod joy;
pub mod net;
pub mod util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        crate::net::server::ServerConfigBuilder::default();
    }
}
