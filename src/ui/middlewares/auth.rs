use crate::{
    core::auth::jwt::{Claims, verify_token},
    error::AppError,
    state::SharedState,
};
use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use std::future::Future;

pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    SharedState: FromRef<S>,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let state = SharedState::from_ref(state);
        async move {
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| AppError::BadRequest("Missing Authorization header".to_string()))?;

            let claims = verify_token(bearer.token(), &state.config.jwt_secret)
                .map_err(|_| AppError::BadRequest("Invalid access token".to_string()))?;

            Ok(AuthUser(claims))
        }
    }
}

#[allow(dead_code)]
pub struct OptionalAuthUser(pub Option<Claims>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
    SharedState: FromRef<S>,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let state = SharedState::from_ref(state);
        async move {
            let Ok(TypedHeader(Authorization(bearer))) =
                parts.extract::<TypedHeader<Authorization<Bearer>>>().await
            else {
                return Ok(OptionalAuthUser(None));
            };

            match verify_token(bearer.token(), &state.config.jwt_secret) {
                Ok(claims) => Ok(OptionalAuthUser(Some(claims))),
                Err(_) => Ok(OptionalAuthUser(None)),
            }
        }
    }
}
