use crate::schema::users;
use crate::{
    core::auth::jwt::{TokenType, create_token, UserRole},
    error::AppError,
    state::SharedState,
};
use super::model::{AuthResponse, NewUser, SigninRequest, SignupRequest, User, UserInfo};
use axum_extra::extract::cookie::{Cookie, SameSite};
use bcrypt::{hash, verify};
use diesel::prelude::*;

#[tracing::instrument(skip(state, payload))]
pub async fn signup_user(state: SharedState, payload: SignupRequest) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    tracing::info!("Signing up user: {}", payload.username);

    // Check if user already exists
    let existing_user = users::table
        .filter(users::email.eq(&payload.email).or(users::username.eq(&payload.username)))
        .first::<User>(&mut conn)
        .optional()
        .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::BadRequest("User with this email or username already exists".to_string()));
    }

    let hashed_password = hash(payload.password, 10)
        .map_err(|_| AppError::InternalServerError("Failed to hash password".to_string()))?;

    let new_user = NewUser {
        username: payload.username,
        email: payload.email,
        password_hash: hashed_password,
        role: UserRole::User,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to create user: {}", e)))?;

    Ok(())
}

#[tracing::instrument(skip(state, payload))]
pub async fn signin_user(
    state: SharedState,
    payload: SigninRequest,
) -> Result<(AuthResponse, Cookie<'static>), AppError> {
    tracing::info!("Signing in user: {}", payload.identifier);
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let user = users::table
        .filter(users::email.eq(&payload.identifier).or(users::username.eq(&payload.identifier)))
        .first::<User>(&mut conn)
        .map_err(|_| AppError::BadRequest("Invalid identifier or password".to_string()))?;

    if !verify(payload.password, &user.password_hash)
        .map_err(|_| AppError::InternalServerError("Password verification failed".to_string()))?
    {
        return Err(AppError::BadRequest(
            "Invalid email or password".to_string(),
        ));
    }

    let created_at = user
        .created_at
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());
    let updated_at = user
        .updated_at
        .unwrap_or_else(|| chrono::Utc::now().naive_utc());

    let access_token = create_token(
        user.id,
        user.username.clone(),
        user.email.clone(),
        user.role.clone(),
        created_at,
        updated_at,
        &state.config.jwt_secret,
        TokenType::Access,
    )?;

    let refresh_token = create_token(
        user.id,
        user.username.clone(),
        user.email.clone(),
        user.role.clone(),
        created_at,
        updated_at,
        &state.config.jwt_refresh_secret,
        TokenType::Refresh,
    )?;

    let refresh_cookie = Cookie::build(("refresh_token", refresh_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        role: user.role,
        created_at,
        updated_at,
    };

    Ok((
        AuthResponse {
            access_token,
            user: user_info,
        },
        refresh_cookie,
    ))
}
