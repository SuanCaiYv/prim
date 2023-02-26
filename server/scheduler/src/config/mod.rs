use std::net::ToSocketAddrs;
use std::{fs, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Context;
use lazy_static::lazy_static;
use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    performance: Option<Performance0>,
    transport: Option<Transport0>,
    redis: Option<Redis0>,
    cluster: Option<Cluster0>,
    rpc: Option<Rpc0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) performance: Performance,
    pub(crate) transport: Transport,
    pub(crate) redis: Redis,
    #[allow(unused)]
    pub(crate) cluster: Cluster,
    pub(crate) rpc: Rpc,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    cluster_address: Option<String>,
    service_address: Option<String>,
    ipv4_type: Option<bool>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) cluster_address: SocketAddr,
    pub(crate) service_address: SocketAddr,
    #[allow(unused)]
    pub(crate) ipv4_type: bool,
    #[allow(unused)]
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
    pub(crate) key: rustls::PrivateKey,
    pub(crate) max_connections: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Performance0 {
    max_sender_side_channel_size: Option<usize>,
    max_receiver_side_channel_size: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Performance {
    pub(crate) max_sender_side_channel_size: usize,
    pub(crate) max_receiver_side_channel_size: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Transport0 {
    keep_alive_interval: Option<u64>,
    connection_idle_timeout: Option<u64>,
    max_bi_streams: Option<usize>,
    max_uni_streams: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    #[allow(unused)]
    pub(crate) keep_alive_interval: Duration,
    pub(crate) connection_idle_timeout: u64,
    pub(crate) max_bi_streams: usize,
    pub(crate) max_uni_streams: usize,
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
struct Cluster0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Cluster {
    #[allow(unused)]
    pub(crate) addresses: Vec<SocketAddr>,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct RpcAPI0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcAPI {
    pub(crate) addresses: Vec<SocketAddr>,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Rpc0 {
    address: Option<String>,
    key_path: Option<String>,
    cert_path: Option<String>,
    api: Option<RpcAPI0>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    pub(crate) address: SocketAddr,
    pub(crate) key: Vec<u8>,
    pub(crate) cert: Vec<u8>,
    pub(crate) api: RpcAPI,
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
            performance: Performance::from_performance0(config0.performance.unwrap()),
            transport: Transport::from_transport0(config0.transport.unwrap()),
            redis: Redis::from_redis0(config0.redis.unwrap()),
            cluster: Cluster::from_scheduler0(config0.cluster.unwrap()),
            rpc: Rpc::from_rpc0(config0.rpc.unwrap()),
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
            cluster_address: server0
                .cluster_address
                .unwrap()
                .to_socket_addrs()
                .expect("parse cluster address failed")
                .collect::<Vec<SocketAddr>>()[0],
            service_address: server0
                .service_address
                .unwrap()
                .to_socket_addrs()
                .expect("parse service address failed")
                .collect::<Vec<SocketAddr>>()[0],
            ipv4_type: server0.ipv4_type.unwrap(),
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
            max_sender_side_channel_size: performance0.max_sender_side_channel_size.unwrap()
                as usize,
            max_receiver_side_channel_size: performance0.max_receiver_side_channel_size.unwrap()
                as usize,
        }
    }
}

impl Transport {
    fn from_transport0(transport0: Transport0) -> Self {
        Transport {
            keep_alive_interval: Duration::from_millis(transport0.keep_alive_interval.unwrap()),
            connection_idle_timeout: transport0.connection_idle_timeout.unwrap(),
            max_bi_streams: transport0.max_bi_streams.unwrap(),
            max_uni_streams: transport0.max_uni_streams.unwrap(),
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

impl Cluster {
    fn from_scheduler0(scheduler0: Cluster0) -> Self {
        let mut addr = vec![];
        for address in scheduler0.addresses.as_ref().unwrap().iter() {
            addr.push(
                address
                    .to_socket_addrs()
                    .expect("parse scheduler address failed")
                    .collect::<Vec<SocketAddr>>()[0],
            );
        }
        let cert = fs::read(PathBuf::from(scheduler0.cert_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Cluster {
            addresses: addr,
            domain: scheduler0.domain.as_ref().unwrap().to_string(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        let key = fs::read(PathBuf::from(rpc0.key_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        let cert = fs::read(PathBuf::from(rpc0.cert_path.as_ref().unwrap()))
            .context("read cert file failed.")
            .unwrap();
        Rpc {
            address: rpc0
                .address
                .unwrap()
                .to_socket_addrs()
                .expect("parse rpc address failed")
                .collect::<Vec<SocketAddr>>()[0],
            key,
            cert,
            api: RpcAPI::from_rpc_api0(rpc0.api.unwrap()),
        }
    }
}

impl RpcAPI {
    fn from_rpc_api0(rpc_api0: RpcAPI0) -> Self {
        let mut addr = vec![];
        for address in rpc_api0.addresses.as_ref().unwrap().iter() {
            addr.push(
                address
                    .to_socket_addrs()
                    .expect("parse rpc api address failed")
                    .collect::<Vec<SocketAddr>>()[0],
            );
        }
        RpcAPI {
            addresses: addr,
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

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe { CONFIG_FILE_PATH }).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

pub(crate) static mut CONFIG_FILE_PATH: &'static str = "config.toml";

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
