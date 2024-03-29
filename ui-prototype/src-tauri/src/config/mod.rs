use std::{fs, path::PathBuf, time::Duration};

use tracing::Level;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    transport: Option<Transport0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) transport: Transport,
}

#[derive(serde::Deserialize, Debug)]
struct Server0 {
    domain: Option<String>,
    cert_path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) domain: String,
    pub(crate) cert: rustls::Certificate,
}

#[derive(serde::Deserialize, Debug)]
struct Transport0 {
    keep_alive_interval: Option<u64>,
    max_bi_streams: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    pub(crate) keep_alive_interval: Duration,
    pub(crate) max_bi_streams: usize,
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
        }
    }
}

impl Server {
    fn from_server0(server0: Server0) -> Self {
        let cert = fs::read(PathBuf::from(server0.cert_path.as_ref().unwrap())).unwrap_or(vec![]);
        Server {
            domain: server0.domain.unwrap(),
            cert: rustls::Certificate(cert),
        }
    }
}

impl Transport {
    fn from_transport0(transport0: Transport0) -> Self {
        Transport {
            keep_alive_interval: Duration::from_millis(transport0.keep_alive_interval.unwrap()),
            max_bi_streams: transport0.max_bi_streams.unwrap(),
        }
    }
}

pub(crate) fn load_config(config_path: &str) {
    let toml_str = fs::read_to_string(config_path).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    let mut config = Config::from_config0(config0);
    if config.server.cert.0.is_empty() {
        let cert_path = PathBuf::from(config_path)
            .parent()
            .unwrap()
            .join("PrimRootCA.crt.der");
        let cert = fs::read(cert_path).unwrap();
        config.server.cert = rustls::Certificate(cert);
    }
    unsafe {
        CONFIG = Some(config);
    }
}

static mut CONFIG: Option<Config> = None;

pub(crate) fn conf() -> &'static Config {
    unsafe { CONFIG.as_ref().unwrap() }
}
