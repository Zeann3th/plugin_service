use crate::schema::users;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use crate::core::auth::jwt::UserRole;

use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(length(min = 3, max = 50, message = "Username must be between 3 and 50 characters"))]
    #[validate(regex(path = "*crate::core::user::model::RE_USERNAME", message = "Username can only contain letters, numbers, and underscores"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
}

use once_cell::sync::Lazy;
use regex::Regex;

pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());

#[derive(Deserialize, Validate)]
pub struct SigninRequest {
    #[serde(alias = "email")]
    #[validate(length(min = 1, message = "Identifier is required"))]
    pub identifier: String, // Can be email or username
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub role: UserRole,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
}
