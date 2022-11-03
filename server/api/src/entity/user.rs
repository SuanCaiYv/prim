use crate::config::CONFIG;
use crate::sql::get_sql_pool;
use chrono::{DateTime, Local};
use common::Result;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow, Default)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) account_id: String,
    pub(crate) credential: String,
    pub(crate) salt: String,
    pub(crate) nickname: String,
    pub(crate) signature: String,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: DateTime<Local>,
}

impl User {
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO $1 (account_id, credential, salt, nickname, signature, create_at, update_at) VALUES ($2, $3, $4, $5, $6, $7, $8)")
            .bind(&format!("{}.user", CONFIG.sql.schema))
            .bind(&self.user_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.signature)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .execute(get_sql_pool())
            .await?;
        Ok(())
    }

    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE $1 SET account_id = $2, credential = $3, salt = $4, nickname = $5, signature = $6, create_at = $7, update_at = $8 WHERE id = $9")
            .bind(&format!("{}.user", CONFIG.sql.schema))
            .bind(&self.user_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.signature)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .bind(&self.id)
            .execute(get_sql_pool())
            .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("UPDATE $1 SET delete_at = $2 WHERE id = $3")
            .bind(&format!("{}.user", CONFIG.sql.schema))
            .bind(Local::now())
            .bind(&self.id)
            .execute(get_sql_pool())
            .await?;
        Ok(())
    }

    pub(crate) async fn get(id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT * FROM $1 WHERE id = $2")
            .bind(&format!("{}.user", CONFIG.sql.schema))
            .bind(id)
            .fetch_one(get_sql_pool())
            .await?;
        Ok(user)
    }

    pub(crate) async fn get_account_id(account_id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT * FROM $1 WHERE account_id = $2")
            .bind(&format!("{}.user", CONFIG.sql.schema))
            .bind(account_id)
            .fetch_one(get_sql_pool())
            .await?;
        Ok(user)
    }
}
