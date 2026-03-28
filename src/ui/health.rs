use crate::{
    state::SharedState,
    error::{ApiResponse, ErrorType, AppError},
};
use axum::{Router, extract::State, routing::get};
use diesel::prelude::*;

pub fn router() -> Router<SharedState> {
    Router::new().route("/healthz", get(health_check))
}

async fn health_check(State(state): State<SharedState>) -> Result<ApiResponse<()>, AppError> {
    let mut conn = state.db_pool.get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let is_alive = diesel::sql_query("SELECT 1")
        .execute(&mut conn)
        .is_ok();

    if !is_alive {
        return Err(AppError::DatabaseError("Database query failed".to_string()));
    }

    Ok(ApiResponse {
        message: "Service is healthy, database connection is active".to_string(),
        error_type: ErrorType::Sucess,
        data: None,
    })
}
