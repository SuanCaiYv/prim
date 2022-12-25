use std::fmt::Display;

use crate::sql::{get_sql_pool, DELETE_AT};
use chrono::{DateTime, Local};
use lib::Result;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow, Default)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) account_id: i64,
    pub(crate) credential: String,
    pub(crate) salt: String,
    pub(crate) nickname: String,
    pub(crate) avatar: String,
    pub(crate) signature: String,
    pub(crate) status: UserStatus,
    pub(crate) info: serde_json::Value,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: DateTime<Local>,
}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Hash, FromPrimitive,
)]
#[sqlx(type_name = "user_status", rename_all = "snake_case")]
pub enum UserStatus {
    NA = 0,
    Online = 1,
    Busy = 2,
    Away = 3,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::NA
    }
}

impl Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<u8> for UserStatus {
    fn from(value: u8) -> Self {
        let e: Option<UserStatus> = FromPrimitive::from_u8(value);
        match e {
            Some(e) => e,
            None => UserStatus::NA,
        }
    }
}

impl User {
    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO api.user (account_id, credential, salt, nickname, avatar, signature, status, info, create_at, update_at, delete_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
            .bind(&self.account_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.avatar)
            .bind(&self.signature)
            .bind(&self.status)
            .bind(&self.info)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .bind(&*DELETE_AT)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE api.user SET account_id = $1, credential = $2, salt = $3, nickname = $4, avatar = $5, signature = $6, status = $7, info = $8, update_at = $9 WHERE id = $10")
            .bind(&self.account_id)
            .bind(&self.credential)
            .bind(&self.salt)
            .bind(&self.nickname)
            .bind(&self.avatar)
            .bind(&self.signature)
            .bind(&self.status)
            .bind(&self.info)
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("UPDATE api.user SET delete_at = $1 WHERE id = $2")
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn get(id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT id, account_id, credential, salt, nickname, avatar, signature, status, info, create_at, update_at, delete_at FROM api.user WHERE id = $1")
            .bind(&id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user)
    }

    #[allow(unused)]
    pub(crate) async fn get_account_id(account_id: i64) -> Result<Self> {
        let user = sqlx::query_as("SELECT id, account_id, credential, salt, nickname, avatar, signature, status, info, create_at, update_at, delete_at FROM api.user WHERE account_id = $1 AND delete_at = $2")
            .bind(&account_id)
            .bind(&*DELETE_AT)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user)
    }
}
