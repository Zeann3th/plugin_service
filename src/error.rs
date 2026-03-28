use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub message: String,
    pub error_type: ErrorType,
    pub data: Option<T>,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorType {
    Sucess,
    Failure,
}

#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    NotFound(String),
    InternalServerError(String),
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(status = %status, "{}", message);
        } else {
            tracing::warn!(status = %status, "{}", message);
        }

        let body = Json(ApiResponse::<()> {
            message,
            error_type: ErrorType::Failure,
            data: None,
        });

        (status, body).into_response()
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
