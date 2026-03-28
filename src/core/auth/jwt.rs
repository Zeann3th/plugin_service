use chrono::{Duration, Utc, NaiveDateTime};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::error::AppError;
use diesel_derive_enum::DbEnum;

#[derive(Debug, Serialize, Deserialize, Clone, DbEnum, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::UserRole"]
#[DbValueStyle = "SCREAMING_SNAKE_CASE"]
pub enum UserRole {
    User,
    Admin,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32, // user id
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub exp: i64,
}

pub enum TokenType {
    Access,
    Refresh,
}

pub fn create_token(
    user_id: i32,
    username: String,
    email: String,
    role: UserRole,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    secret: &str,
    token_type: TokenType,
) -> Result<String, AppError> {
    let expiration = match token_type {
        TokenType::Access => Utc::now() + Duration::minutes(15),
        TokenType::Refresh => Utc::now() + Duration::days(1),
    };

    let claims = Claims {
        sub: user_id,
        username,
        email,
        role,
        created_at,
        updated_at,
        exp: expiration.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| AppError::InternalServerError(format!("Token generation failed: {}", e)))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::InternalServerError(format!("Invalid token: {}", e)))
}
