use crate::error::{ApiResponse, ErrorType};
use axum::response::{IntoResponse, Response};
use std::any::Any;

pub fn global_panic_handler(err: Box<dyn Any + Send + 'static>) -> Response {
    let details = if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    };

    tracing::error!("PANIC caught: {}", details);

    ApiResponse::<()> {
        message: format!("Internal server error: {}", details),
        error_type: ErrorType::Failure,
        data: None,
    }
    .into_response()
}
