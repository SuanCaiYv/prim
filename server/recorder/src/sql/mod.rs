use crate::config::CONFIG;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::sync::OnceCell;

pub(self) static SQL_POOL: OnceCell<Pool<Postgres>> = OnceCell::const_new();

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
