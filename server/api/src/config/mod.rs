use anyhow::Context;
use lazy_static::lazy_static;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    redis: Option<Redis0>,
    scheduler: Option<Scheduler0>,
    rpc: Option<Rpc0>,
    sql: Option<Sql0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) redis: Redis,
    #[allow(unused)]
    pub(crate) scheduler: Scheduler,
    pub(crate) rpc: Rpc,
    pub(crate) sql: Sql,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    service_address: Option<String>,
    #[allow(unused)]
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) service_address: SocketAddr,
    #[allow(unused)]
    pub(crate) domain: String,
    #[allow(unused)]
    pub(crate) cert: rustls::Certificate,
    #[allow(unused)]
    pub(crate) key: rustls::PrivateKey,
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
struct RpcScheduler0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RpcScheduler {
    pub(crate) addresses: Vec<SocketAddr>,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Rpc0 {
    address: Option<String>,
    key_path: Option<String>,
    cert_path: Option<String>,
    scheduler: Option<RpcScheduler0>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    pub(crate) address: SocketAddr,
    pub(crate) key: Vec<u8>,
    pub(crate) cert: Vec<u8>,
    pub(crate) scheduler: RpcScheduler,
}

#[derive(serde::Deserialize, Debug)]
struct Sql0 {
    address: Option<String>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    max_connections: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Sql {
    pub(crate) address: String,
    pub(crate) database: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) max_connections: u32,
}

impl Config {
    fn from_config0(config0: Config0) -> Config {
        let log_level = match config0.log_level.unwrap().as_str() {
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
                .parse()
                .expect("parse service address failed."),
            domain: server0.domain.unwrap(),
            cert: rustls::Certificate(cert),
            key: rustls::PrivateKey(key),
        }
    }
}

impl Redis {
    fn from_redis0(redis0: Redis0) -> Self {
        let mut addr = vec![];
        for address in redis0.addresses.as_ref().unwrap().iter() {
            addr.push(address.parse().expect("parse redis address failed."));
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

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        Rpc {
            address: rpc0
                .address
                .unwrap()
                .parse()
                .expect("parse rpc address failed."),
            key: fs::read(PathBuf::from(rpc0.key_path.as_ref().unwrap()))
                .context("read key file failed.")
                .unwrap(),
            cert: fs::read(PathBuf::from(rpc0.cert_path.as_ref().unwrap()))
                .context("read cert file failed.")
                .unwrap(),
            scheduler: RpcScheduler::from_rpc_balancer0(rpc0.scheduler.unwrap()),
        }
    }
}

impl Sql {
    fn from_sql0(sql0: Sql0) -> Sql {
        Sql {
            address: sql0.address.unwrap(),
            database: sql0.database.unwrap(),
            username: sql0.username.unwrap(),
            password: sql0.password.unwrap(),
            max_connections: sql0.max_connections.unwrap(),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe {CONFIG_FILE_PATH}).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

pub(crate) static mut CONFIG_FILE_PATH: &'static str = "./message/config.toml";

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
