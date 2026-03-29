use chrono::{Duration, Utc, NaiveDateTime};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, ToSql, Output};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::UserRole)]
pub enum UserRole {
    User,
    Admin,
}

impl ToSql<crate::schema::sql_types::UserRole, Pg> for UserRole {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            UserRole::User => out.write_all(b"USER")?,
            UserRole::Admin => out.write_all(b"ADMIN")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::UserRole, Pg> for UserRole {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        match value.as_bytes() {
            b"USER" => Ok(UserRole::User),
            b"ADMIN" => Ok(UserRole::Admin),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64, // user id
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
    user_id: i64,
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
