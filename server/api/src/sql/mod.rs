use std::{str::FromStr, time::SystemTime};

use crate::config::CONFIG;
use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, Pool, Postgres,
};
use tokio::sync::OnceCell;

pub(self) static SQL_POOL: OnceCell<Pool<Postgres>> = OnceCell::const_new();

lazy_static! {
    /// why we need this? cause union unique with null value is not work in postgresql.
    /// so we define ourself "NULL" time.
    pub(crate) static ref DELETE_AT: DateTime<Local> = DateTime::from(SystemTime::UNIX_EPOCH);
}

pub(super) async fn get_sql_pool() -> &'static Pool<Postgres> {
    SQL_POOL
        .get_or_init(|| async {
            let options = PgConnectOptions::from_str(&format!(
                "postgres://{}:{}@{}/{}",
                CONFIG.sql.username, CONFIG.sql.password, CONFIG.sql.address, CONFIG.sql.database
            ))
            .unwrap();
            let options = options.disable_statement_logging();
            PgPoolOptions::new()
                .max_connections(CONFIG.sql.max_connections)
                .connect_with(options)
                .await
                .unwrap()
        })
        .await
}
