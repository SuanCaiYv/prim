use lib::Result;
use chrono::{DateTime, Local};

use crate::sql::get_sql_pool;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow)]
pub(crate) struct UserRelationship {
    pub(crate) id: i64,
    pub(crate) user_account_id: i64,
    pub(crate) peer_account_id: i64,
    pub(crate) remark: String,
    pub(crate) status: UserRelationshipStatus,
    pub(crate) classification: String,
    pub(crate) tag_list: Vec<String>,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_relationship_status", rename_all = "snake_case")]
pub(crate) enum UserRelationshipStatus {
    Normal = 1,
    Lover = 2,
    BestFriend = 3,
    Deleting = 4,
    Deleted = 5,
    Blocked = 6,
}

impl UserRelationship {
    #[allow(unused)]
    pub(crate) async fn get_id(id: i64) -> Result<UserRelationship> {
        let user = sqlx::query_as("SELECT id, user_account_id, peer_account_id, remark, status, classification, tag_list, create_at, update_at, delete_at FROM api.user_relationship WHERE id = $1")
            .bind(id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user)
    }

    #[allow(unused)]
    pub(crate) async fn get_user_account_id(user_account_id: i64) -> Result<Vec<UserRelationship>> {
        let user = sqlx::query_as("SELECT id, user_account_id, peer_account_id, remark, status, classification, tag_list, create_at, update_at, delete_at FROM api.user_relationship WHERE user_account_id = $1 AND delete_at IS NULL")
            .bind(user_account_id)
            .fetch_all(get_sql_pool().await)
            .await?;
        Ok(user)
    }

    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO api.user_relationship (user_account_id, peer_account_id, remark, status, classification, tag_list, create_at, update_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(&self.user_account_id)
            .bind(&self.peer_account_id)
            .bind(&self.remark)
            .bind(&self.status)
            .bind(&self.classification)
            .bind(&self.tag_list)
            .bind(&Local::now())
            .bind(&Local::now())
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE api.user_relationship SET user_account_id = $1, peer_account_id = $2, remark = $3, status = $4, classification = $5, tag_list = $6, update_at = $7 WHERE id = $8")
            .bind(&self.user_account_id)
            .bind(&self.peer_account_id)
            .bind(&self.remark)
            .bind(&self.status)
            .bind(&self.classification)
            .bind(&self.tag_list)
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("UPDATE api.user_relationship SET delete_at = $1 WHERE id = $2")
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }
}