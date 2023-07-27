use std::{str::FromStr, time::SystemTime};

use crate::config::config;
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
            let mut options = PgConnectOptions::from_str(&format!(
                "postgres://{}:{}@{}/{}",
                config().sql.username, config().sql.password, config().sql.address, config().sql.database
            ))
            .unwrap();
            options.disable_statement_logging();
            PgPoolOptions::new()
                .max_connections(config().sql.max_connections)
                .connect_with(options)
                .await
                .unwrap()
        })
        .await
}
