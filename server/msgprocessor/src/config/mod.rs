use std::{fs, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Context;
use lazy_static::lazy_static;
use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    transport: Option<Transport0>,
    redis: Option<Redis0>,
    scheduler: Option<Scheduler0>,
    message_queue: Option<MessageQueue0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) transport: Transport,
    pub(crate) redis: Redis,
    pub(crate) scheduler: Scheduler,
    pub(crate) message_queue: MessageQueue,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
    pub(crate) key: rustls::PrivateKey,
    pub(crate) max_connections: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Transport0 {
    keep_alive_interval: Option<u64>,
    connection_idle_timeout: Option<u64>,
    max_bi_streams: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    pub(crate) keep_alive_interval: Duration,
    pub(crate) connection_idle_timeout: u64,
    pub(crate) max_bi_streams: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Redis0 {
    addresses: Option<Vec<String>>,
}

#[derive(Debug)]
pub(crate) struct Redis {
    pub(crate) addresses: Vec<SocketAddr>,
}

#[derive(serde::Deserialize, Debug)]
struct Scheduler0 {
    address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Scheduler {
    pub(crate) address: SocketAddr,
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct MessageQueue0 {
    address: Option<String>,
}

#[derive(Debug)]
pub(crate) struct MessageQueue {
    pub(crate) address: String,
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
            server: Server::from_server0(config0.server.unwrap()),
            transport: Transport::from_transport0(config0.transport.unwrap()),
            redis: Redis::from_redis0(config0.redis.unwrap()),
            scheduler: Scheduler::from_scheduler0(config0.scheduler.unwrap()),
            message_queue: MessageQueue::from_message_queue0(config0.message_queue.unwrap()),
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
            domain: server0.domain.unwrap(),
            cert: rustls::Certificate(cert),
            key: rustls::PrivateKey(key),
            max_connections: server0.max_connections.unwrap(),
        }
    }
}

impl Transport {
    fn from_transport0(transport0: Transport0) -> Self {
        Transport {
            keep_alive_interval: Duration::from_millis(transport0.keep_alive_interval.unwrap()),
            connection_idle_timeout: transport0.connection_idle_timeout.unwrap(),
            max_bi_streams: transport0.max_bi_streams.unwrap(),
        }
    }
}

impl Redis {
    fn from_redis0(redis0: Redis0) -> Self {
        let mut addr = vec![];
        for address in redis0.addresses.as_ref().unwrap().iter() {
            addr.push(
                address
                    .parse::<SocketAddr>()
                    .expect("parse redis address failed"),
            );
        }
        Redis { addresses: addr }
    }
}

impl Scheduler {
    fn from_scheduler0(mut scheduler0: Scheduler0) -> Self {
        let cert = fs::read(PathBuf::from(scheduler0.cert_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Scheduler {
            address: scheduler0
                .address
                .unwrap()
                .parse::<SocketAddr>()
                .expect("parse scheduler address failed"),
            domain: scheduler0.domain.take().unwrap(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl MessageQueue {
    fn from_message_queue0(message_queue0: MessageQueue0) -> Self {
        MessageQueue {
            address: message_queue0.address.unwrap(),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe { CONFIG_FILE_PATH }).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

pub(crate) static mut CONFIG_FILE_PATH: &'static str = "./msgprocessor/config.toml";

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
