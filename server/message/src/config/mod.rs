use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
    time::Duration,
};

use anyhow::Context;
use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    transport: Option<Transport0>,
    redis: Option<Redis0>,
    scheduler: Option<Scheduler0>,
    rpc: Option<Rpc0>,
    seqnum: Option<Seqnum0>,
    message_queue: Option<MessageQueue0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) transport: Transport,
    pub(crate) redis: Redis,
    pub(crate) scheduler: Scheduler,
    pub(crate) rpc: Rpc,
    pub(crate) seqnum: Seqnum,
    pub(crate) message_queue: MessageQueue,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    ip_version: Option<String>,
    public_service: Option<bool>,
    cluster_address: Option<String>,
    service_address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    // true for ipv4
    pub(crate) ipv4: bool,
    pub(crate) public_service: bool,
    // why not using SocketAddr?
    // when worked with docker compose, DNS resolve will fail.
    // so we use String here, and put off the resolving of domain to the peer.
    pub(crate) cluster_address: String,
    pub(crate) service_address: String,
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
struct RpcScheduler0 {
    address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcScheduler {
    pub(crate) address: SocketAddr,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct RpcAPI0 {
    address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcAPI {
    pub(crate) address: SocketAddr,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Rpc0 {
    scheduler: Option<RpcScheduler0>,
    api: Option<RpcAPI0>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    pub(crate) scheduler: RpcScheduler,
    pub(crate) api: RpcAPI,
}

#[derive(serde::Deserialize, Debug)]
struct Seqnum0 {
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Seqnum {
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
            rpc: Rpc::from_rpc0(config0.rpc.unwrap()),
            seqnum: Seqnum::from_seqnum0(config0.seqnum.unwrap()),
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
            ipv4: server0.ip_version.unwrap() == "v4",
            public_service: server0.public_service.unwrap(),
            cluster_address: server0.cluster_address.unwrap(),
            service_address: server0.service_address.unwrap(),
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
                    .to_socket_addrs()
                    .expect("parse redis address failed")
                    .collect::<Vec<SocketAddr>>()[0],
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
                .to_socket_addrs()
                .expect("parse scheduler address failed")
                .collect::<Vec<SocketAddr>>()[0],
            domain: scheduler0.domain.take().unwrap(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl RpcScheduler {
    fn from_rpc_scheduler0(rpc_scheduler0: RpcScheduler0) -> Self {
        RpcScheduler {
            address: rpc_scheduler0
                .address
                .unwrap()
                .to_socket_addrs()
                .expect("parse rpc scheduler address failed")
                .collect::<Vec<SocketAddr>>()[0],
            domain: rpc_scheduler0.domain.as_ref().unwrap().to_string(),
            cert: tonic::transport::Certificate::from_pem(
                fs::read(PathBuf::from(rpc_scheduler0.cert_path.as_ref().unwrap()))
                    .context("read key file failed.")
                    .unwrap()
                    .as_slice(),
            ),
        }
    }
}

impl RpcAPI {
    fn from_rpc_api0(rpc_api0: RpcAPI0) -> Self {
        RpcAPI {
            address: rpc_api0
                .address
                .unwrap()
                .to_socket_addrs()
                .expect("parse rpc api address failed")
                .collect::<Vec<SocketAddr>>()[0],
            domain: rpc_api0.domain.as_ref().unwrap().to_string(),
            cert: tonic::transport::Certificate::from_pem(
                fs::read(PathBuf::from(rpc_api0.cert_path.as_ref().unwrap()))
                    .context("read key file failed.")
                    .unwrap()
                    .as_slice(),
            ),
        }
    }
}

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        Rpc {
            scheduler: RpcScheduler::from_rpc_scheduler0(rpc0.scheduler.unwrap()),
            api: RpcAPI::from_rpc_api0(rpc0.api.unwrap()),
        }
    }
}

impl Seqnum {
    fn from_seqnum0(seqnum0: Seqnum0) -> Self {
        let cert = fs::read(PathBuf::from(seqnum0.cert_path.as_ref().unwrap()))
            .context("read cert file failed.")
            .unwrap();
        Seqnum {
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

pub(crate) fn load_config(config_path: &str) {
    let toml_str = fs::read_to_string(config_path).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    let mut config = Config::from_config0(config0);
    if let Ok(address) = std::env::var("CLUSTER_ADDRESS") {
        config.server.cluster_address = address;
    }
    if let Ok(address) = std::env::var("SERVICE_ADDRESS") {
        config.server.service_address = address;
    }
    unsafe { CONFIG.replace(config) };
}

pub(self) static mut CONFIG: Option<Config> = None;

pub(crate) fn config() -> &'static Config {
    unsafe { CONFIG.as_ref().unwrap() }
}
