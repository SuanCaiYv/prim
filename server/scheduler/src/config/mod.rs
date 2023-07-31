use std::net::ToSocketAddrs;
use std::{fs, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Context;
use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    transport: Option<Transport0>,
    redis: Option<Redis0>,
    cluster: Option<Cluster0>,
    rpc: Option<Rpc0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) transport: Transport,
    pub(crate) redis: Redis,
    pub(crate) cluster: Cluster,
    pub(crate) rpc: Rpc,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    cluster_address: Option<String>,
    service_address: Option<String>,
    service_ip: Option<String>,
    cluster_ip: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) cluster_address: SocketAddr,
    pub(crate) service_address: SocketAddr,
    pub(crate) service_ip: String,
    pub(crate) cluster_ip: String,
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
struct Cluster0 {
    addresses: Option<Vec<String>>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Cluster {
    pub(crate) addresses: Vec<SocketAddr>,
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
            service_ip: server0.service_ip.unwrap(),
            cluster_ip: server0.cluster_ip.unwrap(),
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

pub(crate) fn load_config(config_path: &str) {
    let toml_str = fs::read_to_string(config_path).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    let mut config = Config::from_config0(config0);
    if let Ok(ip) = std::env::var("OUTER_IP") {
        config.server.service_ip = ip;
    }
    if let Ok(ip) = std::env::var("INNER_IP") {
        config.server.cluster_ip = ip;
    }
    unsafe { CONFIG.replace(config) };
}

pub(self) static mut CONFIG: Option<Config> = None;

pub(crate) fn config() -> &'static Config {
    unsafe { CONFIG.as_ref().unwrap() }
}
