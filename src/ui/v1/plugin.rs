use crate::core::plugin::{model::*, service};
use crate::error::{ApiResponse, AppError, ErrorType};
use crate::state::SharedState;
use crate::ui::middlewares::auth::AuthUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", post(create_plugin).get(get_plugins))
        .route(
            "/{id}",
            get(get_plugin).patch(update_plugin).delete(delete_plugin),
        )
        .route("/{id}/vote", post(vote_plugin))
        .route("/{id}/download/{filename}", get(download_plugin))
}

async fn create_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<CreatePluginRequest>,
) -> Result<ApiResponse<CreatePluginResponse>, AppError> {
    let data = service::create_plugin(state, claims, payload).await?;
    Ok(ApiResponse {
        message: "Plugin metadata created, use upload_url to upload file".to_string(),
        error_type: ErrorType::Sucess,
        data: Some(data),
    })
}

async fn get_plugins(
    State(state): State<SharedState>,
    Query(query): Query<PluginQuery>,
) -> Result<ApiResponse<PaginatedResponse<PluginResponse>>, AppError> {
    let data = service::get_plugins(state, query).await?;
    Ok(ApiResponse {
        message: "Plugins retrieved successfully".to_string(),
        error_type: ErrorType::Sucess,
        data: Some(data),
    })
}

async fn get_plugin(
    State(state): State<SharedState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<PluginResponse>, AppError> {
    let data = service::get_plugin_by_id(state, id).await?;
    Ok(ApiResponse {
        message: "Plugin retrieved successfully".to_string(),
        error_type: ErrorType::Sucess,
        data: Some(data),
    })
}

async fn update_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdatePluginRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::update_plugin(state, claims, id, payload).await?;
    Ok(ApiResponse {
        message: "Plugin updated successfully".to_string(),
        error_type: ErrorType::Sucess,
        data: None,
    })
}

async fn delete_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<()>, AppError> {
    service::delete_plugin(state, claims, id).await?;
    Ok(ApiResponse {
        message: "Plugin deleted successfully".to_string(),
        error_type: ErrorType::Sucess,
        data: None,
    })
}

async fn vote_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i32>,
    Json(payload): Json<VoteRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::vote_plugin(state, claims, id, payload).await?;
    Ok(ApiResponse {
        message: "Vote recorded successfully".to_string(),
        error_type: ErrorType::Sucess,
        data: None,
    })
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DownloadParams {
    filename: String,
}

async fn download_plugin(
    State(state): State<SharedState>,
    Path((id, filename)): Path<(i32, String)>,
) -> Result<ApiResponse<String>, AppError> {
    let url = service::download_plugin(state, id, filename).await?;
    Ok(ApiResponse {
        message: "Presigned download URL generated".to_string(),
        error_type: ErrorType::Sucess,
        data: Some(url),
    })
}
