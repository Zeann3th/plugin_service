use crate::schema::plugins;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePluginRequest {
    #[validate(length(min = 3, max = 50, message = "Code must be between 3 and 50 characters"))]
    #[validate(regex(path = "*crate::core::plugin::model::RE_CODE", message = "Code must contain only alphanumeric characters and hyphens"))]
    pub code: String,
    #[validate(length(min = 3, max = 100, message = "Name must be between 3 and 100 characters"))]
    pub name: String,
    #[validate(length(max = 1000, message = "Description must not exceed 1000 characters"))]
    pub description: Option<String>,
    #[validate(length(min = 1, max = 20, message = "Version must be between 1 and 20 characters"))]
    #[validate(regex(path = "*crate::core::plugin::model::RE_VERSION", message = "Invalid version format (e.g., 1.0.0)"))]
    pub version: String,
    #[validate(length(min = 1, message = "Filename is required"))]
    pub filename: String,
    #[validate(range(min = 1, max = 220200960, message = "File size must be between 1 byte and 210MB"))]
    pub file_size: i64,
}

use once_cell::sync::Lazy;
use regex::Regex;

pub static RE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9-]+$").unwrap());
pub static RE_VERSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+(\.\d+)*(-[a-zA-Z0-9.]+)?$").unwrap());

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdatePluginRequest {
    #[validate(length(min = 3, max = 100, message = "Name must be between 3 and 100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 1000, message = "Description must not exceed 1000 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginResponse {
    pub id: i64,
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
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePluginResponse {
    pub plugin_id: i64,
    pub upload_url: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(table_name = plugins)]
pub struct Plugin {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub publisher_id: i64,
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
    pub publisher_id: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PluginQuery {
    #[validate(length(max = 50))]
    pub code: Option<String>,
    #[validate(length(max = 100))]
    pub name: Option<String>,
    #[validate(range(min = 1, message = "Page must be at least 1"))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100, message = "Per page must be between 1 and 100"))]
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VoteRequest {
    pub is_upvote: bool,
}
