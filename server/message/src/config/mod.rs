use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use quinn::VarInt;
use structopt::lazy_static::lazy_static;
use tracing::Level;

#[derive(serde_derive::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    transport: Option<Transport0>,
    redis: Option<Redis0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) transport: Transport,
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

#[derive(serde_derive::Deserialize, Debug)]
struct Transport0 {
    keep_alive_interval: Option<u64>,
    connection_idle_timeout: Option<u64>,
    max_bi_streams: Option<u64>,
    max_uni_streams: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    pub(crate) keep_alive_interval: VarInt,
    pub(crate) connection_idle_timeout: VarInt,
    pub(crate) max_bi_streams: VarInt,
    pub(crate) max_uni_streams: VarInt,
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
        let log_level = match config0.log_level.unwrap_or("info".to_string()).as_ref() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };
        Config {
            log_level,
            server: Server::from_server0(&config0.server.unwrap()),
            transport: Transport::from_transport0(&config0.transport.unwrap()),
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

impl Transport {
    fn from_transport0(transport0: &Transport0) -> Self {
        Transport {
            keep_alive_interval: VarInt::from_u64(transport0.keep_alive_interval.unwrap()).unwrap(),
            connection_idle_timeout: VarInt::from_u64(transport0.connection_idle_timeout.unwrap()).unwrap(),
            max_bi_streams: VarInt::from_u64(transport0.max_bi_streams.unwrap()).unwrap(),
            max_uni_streams: VarInt::from_u64(transport0.max_uni_streams.unwrap()).unwrap(),
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