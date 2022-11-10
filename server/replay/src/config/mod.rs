use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use lazy_static::lazy_static;
use quinn::VarInt;
use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    max_deal_time: Option<u64>,
    cluster_addresses: Option<Vec<String>>,
    server: Option<Server0>,
    performance: Option<Performance0>,
    transport: Option<Transport0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) max_deal_time: Duration,
    pub(crate) cluster_addresses: Vec<SocketAddr>,
    pub(crate) server: Server,
    #[allow(unused)]
    pub(crate) performance: Performance,
    pub(crate) transport: Transport,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    inner_address: Option<String>,
    outer_address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) inner_address: SocketAddr,
    pub(crate) outer_address: SocketAddr,
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
    pub(crate) key: rustls::PrivateKey,
    pub(crate) max_connections: u32,
}

#[derive(serde::Deserialize, Debug)]
struct Performance0 {
    max_sender_side_channel_size: Option<u64>,
    max_receiver_side_channel_size: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct Performance {
    #[allow(unused)]
    pub(crate) max_sender_side_channel_size: usize,
    #[allow(unused)]
    pub(crate) max_receiver_side_channel_size: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Transport0 {
    keep_alive_interval: Option<u64>,
    connection_idle_timeout: Option<u64>,
    max_bi_streams: Option<u64>,
    max_uni_streams: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    pub(crate) keep_alive_interval: Duration,
    pub(crate) connection_idle_timeout: VarInt,
    pub(crate) max_bi_streams: VarInt,
    pub(crate) max_uni_streams: VarInt,
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
            max_deal_time: Duration::from_millis(config0.max_deal_time.unwrap_or(3000)),
            cluster_addresses: config0
                .cluster_addresses
                .unwrap_or(vec![])
                .into_iter()
                .map(|address| SocketAddr::from_str(&address).unwrap())
                .collect(),
            server: Server::from_server0(config0.server.unwrap()),
            performance: Performance::from_performance0(config0.performance.unwrap()),
            transport: Transport::from_transport0(config0.transport.unwrap()),
        }
    }
}

impl Server {
    fn from_server0(server0: Server0) -> Self {
        let cert = fs::read(PathBuf::from(server0.cert_path.as_ref().unwrap()))
            .context("read cert file failed.")
            .unwrap();
        let key = fs::read(PathBuf::from(server0.key_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Server {
            inner_address: SocketAddr::from_str(server0.inner_address.as_ref().unwrap()).unwrap(),
            outer_address: SocketAddr::from_str(server0.outer_address.as_ref().unwrap()).unwrap(),
            domain: server0.domain.unwrap(),
            cert: rustls::Certificate(cert),
            key: rustls::PrivateKey(key),
            max_connections: server0.max_connections.unwrap(),
        }
    }
}

impl Performance {
    fn from_performance0(performance0: Performance0) -> Self {
        Performance {
            max_sender_side_channel_size: performance0.max_sender_side_channel_size.unwrap() as usize,
            max_receiver_side_channel_size: performance0.max_receiver_side_channel_size.unwrap() as usize,
        }
    }
}

impl Transport {
    fn from_transport0(transport0: Transport0) -> Self {
        Transport {
            keep_alive_interval: Duration::from_millis(transport0.keep_alive_interval.unwrap()),
            connection_idle_timeout: VarInt::from_u64(transport0.connection_idle_timeout.unwrap())
                .unwrap(),
            max_bi_streams: VarInt::from_u64(transport0.max_bi_streams.unwrap()).unwrap(),
            max_uni_streams: VarInt::from_u64(transport0.max_uni_streams.unwrap()).unwrap(),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe { CONFIG_FILE_PATH }).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

pub(crate) static mut CONFIG_FILE_PATH: &'static str = "config.toml";

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
