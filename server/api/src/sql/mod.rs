use std::time::SystemTime;

use crate::config::CONFIG;
use chrono::{Local, DateTime};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::sync::OnceCell;
use lazy_static::lazy_static;

pub(self) static SQL_POOL: OnceCell<Pool<Postgres>> = OnceCell::const_new();

lazy_static! {
    /// why we need this? cause union unique with null value is not work in postgresql.
    /// so we define ourself's "NULL" time.
    pub(crate) static ref DELETE_AT: DateTime<Local> = DateTime::from(SystemTime::UNIX_EPOCH);
}

pub(super) async fn get_sql_pool() -> &'static Pool<Postgres> {
    SQL_POOL
        .get_or_init(|| async {
            PgPoolOptions::new()
                .max_connections(CONFIG.sql.max_connections)
                .connect(&format!(
                    "postgres://{}:{}@{}/{}",
                    CONFIG.sql.username,
                    CONFIG.sql.password,
                    CONFIG.sql.address,
                    CONFIG.sql.database
                ))
                .await
                .unwrap()
        })
        .await
}
