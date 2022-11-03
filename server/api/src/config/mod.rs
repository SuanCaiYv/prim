use anyhow::Context;
use lazy_static::lazy_static;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use tracing::Level;

#[derive(serde_derive::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    redis: Option<Redis0>,
    rpc: Option<Rpc0>,
    sql: Option<Sql0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) redis: Redis,
    pub(crate) rpc: Rpc,
    pub(crate) sql: Sql,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    address: Option<String>,
    #[allow(unused)]
    domain: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) address: SocketAddr,
    #[allow(unused)]
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
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
struct Rpc0 {
    addresses: Option<Vec<String>>,
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Rpc {
    pub(crate) addresses: Vec<SocketAddr>,
    pub(crate) domain: String,
    pub(crate) cert: tonic::transport::Certificate,
}

#[derive(serde_derive::Deserialize, Debug)]
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
    pub(crate) schema: String,
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
            address: SocketAddr::from_str(server0.address.as_ref().unwrap()).unwrap(),
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
            addr.push(SocketAddr::from_str(address).unwrap());
        }
        Redis { addresses: addr }
    }
}

impl Rpc {
    fn from_rpc0(rpc0: Rpc0) -> Self {
        let mut addr = vec![];
        for address in rpc0.addresses.as_ref().unwrap().iter() {
            addr.push(SocketAddr::from_str(address).unwrap());
        }
        let cert = fs::read(PathBuf::from(rpc0.cert_path.as_ref().unwrap()))
            .context("read key file failed.")
            .unwrap();
        Rpc {
            addresses: addr,
            domain: rpc0.domain.as_ref().unwrap().to_string(),
            cert: tonic::transport::Certificate::from_pem(cert),
        }
    }
}

impl Sql {
    fn from_sql0(sql0: Sql0) -> Sql {
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
    let toml_str = fs::read_to_string("config.toml").unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

lazy_static! {
    pub(crate) static ref CONFIG: Config = load_config();
}
