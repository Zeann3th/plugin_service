use crate::schema::plugins;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePluginRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePluginRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub publisher: UserInfo,
    pub download_count: i32,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePluginResponse {
    pub plugin_id: i32,
    pub upload_url: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(table_name = plugins)]
pub struct Plugin {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub publisher_id: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub download_count: Option<i32>,
    pub upvote_count: Option<i32>,
    pub downvote_count: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = plugins)]
pub struct NewPlugin {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub publisher_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct PluginQuery {
    pub code: Option<String>,
    pub name: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub is_upvote: bool,
}
