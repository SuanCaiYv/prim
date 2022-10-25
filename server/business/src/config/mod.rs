use std::fs;
use lazy_static::lazy_static;

use tracing::Level;

#[derive(serde_derive::Deserialize, Debug)]
struct Config0 {
    log_level: Option<String>,
    sql: Option<Sql0>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) log_level: Level,
    pub(crate) sql: Sql,
}

#[derive(serde_derive::Deserialize, Debug)]
struct Sql0 {
    address: Option<String>,
    database: Option<String>,
    schema: Option<String>,
    username: Option<String>,
    password: Option<String>,
    connection_pool_size: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Sql {
    pub(crate) address: String,
    pub(crate) database: String,
    pub(crate) schema: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) connection_pool_size: u32,
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
            sql: Sql::from_sql0(config0.sql.unwrap()),
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
            connection_pool_size: sql0.connection_pool_size.unwrap(),
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
