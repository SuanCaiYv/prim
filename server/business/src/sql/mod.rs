use crate::config::CONFIG;
use common::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub(crate) static mut SQL_POOL: Option<Pool<Postgres>> = None;

pub(super) async fn sql_connection_pool() -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(CONFIG.sql.connection_pool_size)
        .connect(&format!(
            "postgres://{}:{}@{}/{}",
            CONFIG.sql.username, CONFIG.sql.password, CONFIG.sql.address, CONFIG.sql.database
        ))
        .await
        .unwrap();
    Ok(pool)
}
