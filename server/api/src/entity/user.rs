use crate::sql::get_sql_pool;
use chrono::{DateTime, Local};
use lib::Result;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow, Default)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) account_id: String,
    pub(crate) credential: String,
    pub(crate) salt: String,
    pub(crate) nickname: String,
    pub(crate) avatar: String,
    pub(crate) signature: String,
    pub(crate) status: UserStatus,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: Option<DateTime<Local>>,

}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Hash,
)]
pub enum UserStatus {
    NA,
    Online,
    Busy,
    Away,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::NA
    }
}

impl User {
    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO api.user (account_id, credential, salt, nickname, signature, create_at, update_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&self.account_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.signature)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE api.user SET account_id = $1, credential = $2, salt = $3, nickname = $4, signature = $5, create_at = $6, update_at = $7 WHERE id = $8")
            .bind(&self.account_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.signature)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("UPDATE api.user SET delete_at = $1 WHERE id = $2")
            .bind(Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn get(id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT id, account_id, credential, salt, nickname, signature, create_at, update_at FROM api.user WHERE id = $1")
            .bind(id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user)
    }

    #[allow(unused)]
    pub(crate) async fn get_account_id(account_id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT id, account_id, credential, salt, nickname, signature, create_at, update_at FROM api.user WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user)
    }
}
