use crate::sql::{get_sql_pool, DELETE_AT};
use chrono::{DateTime, Local};
use lib::Result;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_json::json;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow)]
pub(crate) struct Group {
    pub(crate) id: i64,
    pub(crate) group_id: i64,
    pub(crate) name: String,
    pub(crate) avatar: String,
    pub(crate) admin_list: Vec<serde_json::Value>,
    pub(crate) member_list: Vec<serde_json::Value>,
    pub(crate) status: GroupStatus,
    pub(crate) info: serde_json::Value,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: DateTime<Local>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "group_status", rename_all = "snake_case")]
pub(crate) enum GroupStatus {
    NA = 0,
    Normal = 1,
    Banned = 2,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, sqlx::FromRow)]
pub(crate) struct UserGroupList {
    pub(crate) id: i64,
    pub(crate) user_id: i64,
    pub(crate) group_id: i64,
    pub(crate) role: UserGroupRole,
    pub(crate) create_at: DateTime<Local>,
    pub(crate) update_at: DateTime<Local>,
    pub(crate) delete_at: DateTime<Local>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy, sqlx::Type, PartialEq, Eq, FromPrimitive)]
#[sqlx(type_name = "user_group_role", rename_all = "snake_case")]
pub(crate) enum UserGroupRole {
    NA = 0,
    Member = 1,
    Admin = 2,
}

impl From<u8> for UserGroupRole {
    fn from(v: u8) -> Self {
        let role: Option<UserGroupRole> = FromPrimitive::from_u8(v);
        match role {
            Some(r) => r,
            None => UserGroupRole::NA,
        }
    }
}

impl Default for Group {
    fn default() -> Self {
        Self {
            id: 1,
            group_id: 68719476736,
            name: "test-group".to_string(),
            avatar: "".to_string(),
            admin_list: vec![json!({"user_id": 1, "remark": "user-1"})],
            member_list: vec![json!({"user_id": 2, "remark": "user-2"})],
            status: GroupStatus::Normal,
            info: serde_json::Value::Null,
            create_at: Local::now(),
            update_at: Local::now(),
            delete_at: DELETE_AT.clone(),
        }
    }
}

impl Group {
    pub(crate) async fn get_group_id(group_id: i64) -> Result<Group> {
        let group = sqlx::query_as("SELECT id, group_id, name, avatar, admin_list, member_list, status, info, create_at, update_at, delete_at FROM api.group WHERE group_id = $1 AND delete_at = $2")
            .bind(&group_id)
            .bind(&*DELETE_AT)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(group)
    }

    #[allow(unused)]
    pub(crate) async fn get_by_id(id: i64) -> Result<Group> {
        let group = sqlx::query_as("SELECT id, group_id, name, avatar, admin_list, member_list, status, info, create_at, update_at, delete_at FROM api.group WHERE id = $1")
            .bind(id)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(group)
    }

    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO api.group (group_id, name, avatar, admin_list, member_list, status, info, create_at, update_at, delete_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
            .bind(&self.group_id)
            .bind(&self.name)
            .bind(&self.avatar)
            .bind(&self.admin_list)
            .bind(&self.member_list)
            .bind(&self.status)
            .bind(&self.info)
            .bind(&Local::now())
            .bind(&Local::now())
            .bind(&*DELETE_AT)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE api.group SET group_id = $1, name = $2, avatar = $3, admin_list = $4, member_list = $5, status = $6, info = $7, update_at = $8 WHERE id = $9")
            .bind(&self.group_id)
            .bind(&self.name)
            .bind(&self.avatar)
            .bind(&self.admin_list)
            .bind(&self.member_list)
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
        sqlx::query("UPDATE api.group SET delete_at = $1 WHERE id = $2")
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }
}

impl UserGroupList {
    #[allow(unused)]
    pub(crate) async fn get_by_user_id(user_id: i64) -> Result<Vec<UserGroupList>> {
        let user_group_list = sqlx::query_as("SELECT id, user_id, group_id, role, create_at, update_at, delete_at FROM api.user_group_list WHERE user_id = $1 AND delete_at = $2")
            .bind(&user_id)
            .bind(&*DELETE_AT)
            .fetch_all(get_sql_pool().await)
            .await?;
        Ok(user_group_list)
    }

    #[allow(unused)]
    pub(crate) async fn get_by_group_id(group_id: i64) -> Result<Vec<UserGroupList>> {
        let user_group_list = sqlx::query_as("SELECT id, user_id, group_id, role, create_at, update_at, delete_at FROM api.user_group_list WHERE group_id = $1 AND delete_at = $2")
            .bind(&group_id)
            .bind(&*DELETE_AT)
            .fetch_all(get_sql_pool().await)
            .await?;
        Ok(user_group_list)
    }

    #[allow(unused)]
    pub(crate) async fn get_user_id_group_id(user_id: i64, group_id: i64) -> Result<UserGroupList> {
        let user_group_list = sqlx::query_as("SELECT id, user_id, group_id, role, create_at, update_at, delete_at FROM api.user_group_list WHERE user_id = $1 AND group_id = $2 AND delete_at = $3")
            .bind(&user_id)
            .bind(&group_id)
            .bind(&*DELETE_AT)
            .fetch_one(get_sql_pool().await)
            .await?;
        Ok(user_group_list)
    }

    #[allow(unused)]
    pub(crate) async fn insert(&self) -> Result<()> {
        sqlx::query("INSERT INTO api.user_group_list (user_id, group_id, role, create_at, update_at, delete_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&self.user_id)
            .bind(&self.group_id)
            .bind(&self.role)
            .bind(&self.create_at)
            .bind(&self.update_at)
            .bind(&*DELETE_AT)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn update(&self) -> Result<()> {
        sqlx::query("UPDATE api.user_group_list SET user_id = $1, group_id = $2, role = $3, update_at = $4 WHERE id = $5")
            .bind(&self.user_id)
            .bind(&self.group_id)
            .bind(&self.role)
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub(crate) async fn delete(&self) -> Result<()> {
        sqlx::query("UPDATE api.user_group_list SET delete_at = $1 WHERE id = $2")
            .bind(&Local::now())
            .bind(&self.id)
            .execute(get_sql_pool().await)
            .await?;
        Ok(())
    }
}
