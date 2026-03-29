use crate::error::AppError;
use axum::{
    extract::{FromRequest, Request, Query, FromRequestParts},
    Json as AxumJson,
    http::request::Parts,
};
use serde::de::DeserializeOwned;
use validator::Validate;

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let AxumJson(value) = AxumJson::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        
        value.validate().map_err(|e| AppError::BadRequest(format!("Validation failed: {}", e)))?;
        
        Ok(ValidatedJson(value))
    }
}

pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        
        value.validate().map_err(|e| AppError::BadRequest(format!("Validation failed: {}", e)))?;
        
        Ok(ValidatedQuery(value))
    }
}
