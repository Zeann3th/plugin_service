use crate::schema::{plugins, plugin_versions, tags, plugin_tags, user_plugins};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, Output, ToSql};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::PluginStatus)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginStatus {
    Draft,
    Published,
}

impl ToSql<crate::schema::sql_types::PluginStatus, Pg> for PluginStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            PluginStatus::Draft => out.write_all(b"DRAFT")?,
            PluginStatus::Published => out.write_all(b"PUBLISHED")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::PluginStatus, Pg> for PluginStatus {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        match value.as_bytes() {
            b"DRAFT" => Ok(PluginStatus::Draft),
            b"PUBLISHED" => Ok(PluginStatus::Published),
            _ => Err("Unrecognized plugin_status variant".into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallationStatus {
    NotInstalled,
    Installed,
    Updatable,
}

use once_cell::sync::Lazy;
use regex::Regex;

pub static RE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9-]+$").unwrap());
pub static RE_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d+(\.\d+)*(-[a-zA-Z0-9.]+)?$").unwrap());

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePluginRequest {
    #[validate(length(min = 3, max = 50, message = "Code must be between 3 and 50 characters"))]
    #[validate(regex(
        path = "*crate::core::plugin::model::RE_CODE",
        message = "Code must contain only alphanumeric characters and hyphens"
    ))]
    pub code: String,
    #[validate(length(min = 3, max = 100, message = "Name must be between 3 and 100 characters"))]
    pub name: String,
    #[validate(length(max = 1000, message = "Description must not exceed 1000 characters"))]
    pub description: Option<String>,
    #[validate(length(min = 1, max = 20, message = "Version must be between 1 and 20 characters"))]
    #[validate(regex(
        path = "*crate::core::plugin::model::RE_VERSION",
        message = "Invalid version format (e.g., 1.0.0)"
    ))]
    pub version: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePluginResponse {
    pub plugin_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UploadPluginRequest {
    #[validate(length(min = 1, message = "Filename is required"))]
    pub filename: String,
    #[validate(range(
        min = 1,
        max = 220200960,
        message = "File size must be between 1 byte and 210MB"
    ))]
    pub file_size: i64,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadPluginResponse {
    pub upload_url: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdatePluginRequest {
    #[validate(length(min = 3, max = 100, message = "Name must be between 3 and 100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 1000, message = "Description must not exceed 1000 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginVersionResponse {
    pub version: String,
    pub status: PluginStatus,
    pub download_count: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher: UserInfo,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub tags: Vec<String>,
    pub latest_version: Option<String>,
    pub installation_status: InstallationStatus,
    pub versions: Option<Vec<PluginVersionResponse>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = plugins)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Plugin {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub upvote_count: Option<i32>,
    pub downvote_count: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = plugins)]
pub struct NewPlugin {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = plugin_versions)]
#[diesel(belongs_to(Plugin))]
pub struct PluginVersion {
    pub id: i64,
    pub plugin_id: i64,
    pub version: String,
    pub file_path: Option<String>,
    pub download_count: Option<i32>,
    pub status: PluginStatus,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = plugin_versions)]
pub struct NewPluginVersion {
    pub plugin_id: i64,
    pub version: String,
    pub status: PluginStatus,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = tags)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = tags)]
pub struct NewTag {
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = plugin_tags)]
pub struct NewPluginTag {
    pub plugin_id: i64,
    pub tag_id: i64,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = user_plugins)]
pub struct UserPlugin {
    pub id: i64,
    pub user_id: i64,
    pub plugin_id: i64,
    pub version: String,
    pub downloaded_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = user_plugins)]
pub struct NewUserPlugin {
    pub user_id: i64,
    pub plugin_id: i64,
    pub version: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PluginQuery {
    #[validate(length(max = 50))]
    pub code: Option<String>,
    #[validate(length(max = 100))]
    pub name: Option<String>,
    pub tag: Option<String>,
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
