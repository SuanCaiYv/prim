use std::{fs, path::PathBuf, time::Duration};

use anyhow::Context;
use tracing::Level;

use crate::CONFIG_PATH;

#[derive(serde::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    server: Option<Server0>,
    performance: Option<Performance0>,
    transport: Option<Transport0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) server: Server,
    pub(crate) performance: Performance,
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
    max_bi_streams: Option<usize>,
    max_uni_streams: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Transport {
    pub(crate) keep_alive_interval: Duration,
    pub(crate) max_bi_streams: usize,
    pub(crate) max_uni_streams: usize,
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
        }
    }
}

impl Server {
    fn from_server0(server0: Server0) -> Self {
        let cert = fs::read(PathBuf::from(server0.cert_path.as_ref().unwrap()))
            .context("read cert file failed.")
            .unwrap();
        Server {
            domain: server0.domain.unwrap(),
            cert: rustls::Certificate(cert),
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
            max_bi_streams: transport0.max_bi_streams.unwrap(),
            max_uni_streams: transport0.max_uni_streams.unwrap(),
        }
    }
}

pub(crate) fn load_config() -> Config {
    let toml_str = fs::read_to_string(unsafe { CONFIG_PATH }).unwrap();
    let config0: Config0 = toml::from_str(&toml_str).unwrap();
    Config::from_config0(config0)
}

static mut CONFIG: Option<Config> = None;

pub(crate) fn conf() -> &'static Config {
    unsafe {
        if CONFIG.is_none() {
            CONFIG = Some(load_config());
        }
        CONFIG.as_ref().unwrap()
    }
}
