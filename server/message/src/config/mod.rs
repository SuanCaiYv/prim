use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use structopt::lazy_static::lazy_static;

#[derive(serde_derive::Deserialize, Debug)]
struct Config0 {
    server: Option<Server0>,
    redis: Option<Redis0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) server: Server,
    pub(crate) redis: Redis,
}

#[derive(serde_derive::Deserialize, Debug)]
struct Server0 {
    address: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) address: SocketAddr,
    pub(crate) cert: rustls::Certificate,
    pub(crate) key: rustls::PrivateKey,
    pub(crate) max_connections: u32,
}

struct Transport0 {
}

#[derive(serde_derive::Deserialize, Debug)]
struct Redis0 {
    addresses: Option<Vec<String>>,
}

#[derive(Debug)]
pub(crate) struct Redis {
    pub(crate) addresses: Vec<SocketAddr>,
}

impl Config {
    fn from_config0(config0: Config0) -> Config {
        Config {
            server: Server::from_server0(&config0.server.unwrap()),
            redis: Redis::from_redis0(&config0.redis.unwrap()),
        }
    }
}

impl Server {
    fn from_server0(server0: &Server0) -> Self {
        let cert = fs::read(PathBuf::from(server0.cert_path.as_ref().unwrap())).context("read cert file failed.").unwrap();
        let key = fs::read(PathBuf::from(server0.key_path.as_ref().unwrap())).context("read key file failed.").unwrap();
        Server {
            address: SocketAddr::from_str(server0.address.as_ref().unwrap()).unwrap(),
            cert: rustls::Certificate(cert),
            key: rustls::PrivateKey(key),
            max_connections: server0.max_connections.unwrap()
        }
    }
}

impl Redis {
    fn from_redis0(redis0: &Redis0) -> Self {
        let mut addr = vec![];
        for address in redis0.addresses.as_ref().unwrap().iter() {
            addr.push(SocketAddr::from_str(address).unwrap());
        }
        Redis {
            addresses: addr
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string("config.toml").unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

lazy_static!(
    pub(crate) static ref CONFIG: Config = load_config();
);