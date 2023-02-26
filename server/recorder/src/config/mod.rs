use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
    time::Duration,
};

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
    rpc: Option<Rpc0>,
    sql: Option<Sql0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) performance: Performance,
    pub(crate) transport: Transport,
    pub(crate) redis: Redis,
    pub(crate) scheduler: Scheduler,
    pub(crate) rpc: Rpc,
    pub(crate) sql: Sql,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    service_address: Option<String>,
    ipv4_type: Option<bool>,
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    max_connections: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) service_address: SocketAddr,
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
struct Rpc0 {
    scheduler: Option<RpcScheduler0>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    pub(crate) scheduler: RpcScheduler,
}

#[derive(serde::Deserialize, Debug)]
struct Scheduler0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Scheduler {
    pub(crate) addresses: Vec<SocketAddr>,
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
#[allow(unused)]
struct RpcScheduler0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct RpcScheduler {
    pub(crate) addresses: Vec<SocketAddr>,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Sql0 {
    address: Option<String>,
    database: Option<String>,
    schema: Option<String>,
    username: Option<String>,
    password: Option<String>,
    max_connections: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Sql {
    pub(crate) address: String,
    pub(crate) database: String,
    #[allow(unused)]
    pub(crate) schema: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) max_connections: u32,
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
            rpc: Rpc::from_rpc0(config0.rpc.unwrap()),
            sql: Sql::from_sql0(config0.sql.unwrap()),
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
                    .expect("parse clusredister address failed")
                    .collect::<Vec<SocketAddr>>()[0],
            );
        }
        Redis { addresses: addr }
    }
}

impl Scheduler {
    fn from_scheduler0(scheduler0: Scheduler0) -> Self {
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
            .context("read cert file failed.")
            .unwrap();
        Scheduler {
            addresses: addr,
            domain: scheduler0.domain.as_ref().unwrap().to_string(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl RpcScheduler {
    fn from_rpc_balancer0(rpc_balancer0: RpcScheduler0) -> Self {
        let mut addr = vec![];
        for address in rpc_balancer0.addresses.as_ref().unwrap().iter() {
            addr.push(
                address
                    .to_socket_addrs()
                    .expect("parse rpc scheduler address failed")
                    .collect::<Vec<SocketAddr>>()[0],
            );
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

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        Rpc {
            scheduler: RpcScheduler::from_rpc_balancer0(rpc0.scheduler.unwrap()),
        }
    }
}

impl Sql {
    fn from_sql0(sql0: Sql0) -> Self {
        Sql {
            address: sql0.address.unwrap(),
            database: sql0.database.unwrap(),
            schema: sql0.schema.unwrap(),
            username: sql0.username.unwrap(),
            password: sql0.password.unwrap(),
            max_connections: sql0.max_connections.unwrap(),
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
