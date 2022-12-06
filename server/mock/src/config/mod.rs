use std::{fs, net::SocketAddr, path::PathBuf, str::FromStr, time::Duration};

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
    scheduler: Option<Scheduler0>,
    recorder: Option<Recorder0>,
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
    pub(crate) scheduler: Scheduler,
    #[allow(unused)]
    pub(crate) recorder: Recorder,
    #[allow(unused)]
    pub(crate) rpc: Rpc,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    cluster_address: Option<String>,
    service_address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    #[allow(unused)]
    pub(crate) cluster_address: SocketAddr,
    pub(crate) service_address: SocketAddr,
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
    #[allow(unused)]
    pub(crate) key: rustls::PrivateKey,
    #[allow(unused)]
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
    pub(crate) keep_alive_interval: Duration,
    #[allow(unused)]
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
struct Scheduler0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Scheduler {
    #[allow(unused)]
    pub(crate) addresses: Vec<SocketAddr>,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Recorder0 {
    address: Option<String>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Recorder {
    #[allow(unused)]
    pub(crate) address: SocketAddr,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct RpcScheduler0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcScheduler {
    #[allow(unused)]
    pub(crate) addresses: Vec<SocketAddr>,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct RpcAPI0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcAPI {
    #[allow(unused)]
    pub(crate) addresses: Vec<SocketAddr>,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Rpc0 {
    scheduler: Option<RpcScheduler0>,
    api: Option<RpcAPI0>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    #[allow(unused)]
    pub(crate) scheduler: RpcScheduler,
    #[allow(unused)]
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
            scheduler: Scheduler::from_scheduler0(config0.scheduler.unwrap()),
            recorder: Recorder::from_recorder0(config0.recorder.unwrap()),
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
                .parse()
                .context("parse cluster address failed.")
                .unwrap(),
            service_address: server0
                .service_address
                .unwrap()
                .parse()
                .context("parse service address failed.")
                .unwrap(),
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
            addr.push(SocketAddr::from_str(address).unwrap());
        }
        Redis { addresses: addr }
    }
}

impl Scheduler {
    fn from_scheduler0(balancer0: Scheduler0) -> Self {
        let mut addr = vec![];
        for address in balancer0.addresses.as_ref().unwrap().iter() {
            addr.push(address.parse().expect("parse balancer address failed."));
        }
        let cert = fs::read(PathBuf::from(balancer0.cert_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Scheduler {
            addresses: addr,
            domain: balancer0.domain.as_ref().unwrap().to_string(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl Recorder {
    fn from_recorder0(recorder0: Recorder0) -> Self {
        let cert = fs::read(PathBuf::from(recorder0.cert_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Recorder {
            address: recorder0.address.unwrap().parse().expect("parse balancer address failed."),
            domain: recorder0.domain.as_ref().unwrap().to_string(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl RpcScheduler {
    fn from_rpc_balancer0(rpc_balancer0: RpcScheduler0) -> Self {
        let mut addr = vec![];
        for address in rpc_balancer0.addresses.as_ref().unwrap().iter() {
            addr.push(address.parse().expect("parse balancer address failed."));
        }
        RpcScheduler {
            addresses: addr,
            domain: rpc_balancer0.domain.as_ref().unwrap().to_string(),
            cert: tonic::transport::Certificate::from_pem(
                fs::read(PathBuf::from(rpc_balancer0.cert_path.as_ref().unwrap()))
                    .context("read key file failed.")
                    .unwrap()
                    .as_slice(),
            ),
        }
    }
}

impl RpcAPI {
    fn from_rpc_api0(rpc_api0: RpcAPI0) -> Self {
        let mut addr = vec![];
        for address in rpc_api0.addresses.as_ref().unwrap().iter() {
            addr.push(address.parse().expect("parse balancer address failed."));
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

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        Rpc {
            scheduler: RpcScheduler::from_rpc_balancer0(rpc0.scheduler.unwrap()),
            api: RpcAPI::from_rpc_api0(rpc0.api.unwrap()),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe { CONFIG_FILE_PATH }).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

pub(crate) static mut CONFIG_FILE_PATH: &'static str = "./mock/config.toml";

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
