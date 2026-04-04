use crate::core::plugin::{model::*, service};
use crate::error::{ApiResponse, AppError, ErrorType};
use crate::state::SharedState;
use crate::ui::middlewares::auth::{AuthUser, OptionalAuthUser};
use crate::ui::middlewares::validator::{ValidatedJson, ValidatedQuery};
use axum::{
    Router,
    extract::{Path, State, Query},
    routing::{get, post, patch, delete},
};
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", post(create_plugin).get(get_plugins))
        .route(
            "/{id}",
            get(get_plugin).patch(update_plugin).delete(delete_plugin),
        )
        .route(
            "/{id}/versions/{version}",
            patch(update_plugin_version).delete(delete_plugin_version),
        )
        .route("/{id}/vote", post(vote_plugin))
        .route("/{id}/upload", post(upload_plugin))
        .route("/{id}/publish", post(publish_plugin))
        .route("/{id}/download", get(download_plugin))
}

#[tracing::instrument(skip(state))]
async fn create_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    ValidatedJson(payload): ValidatedJson<CreatePluginRequest>,
) -> Result<ApiResponse<CreatePluginResponse>, AppError> {
    let data = service::create_plugin(state, claims, payload).await?;
    Ok(ApiResponse {
        message: "Plugin created with DRAFT status. Call /upload to attach a file, then /publish to go live.".to_string(),
        error_type: ErrorType::Success,
        data: Some(data),
    })
}

#[tracing::instrument(skip(state))]
async fn upload_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    ValidatedJson(payload): ValidatedJson<UploadPluginRequest>,
) -> Result<ApiResponse<UploadPluginResponse>, AppError> {
    let data = service::get_upload_url(state, claims, id, payload).await?;
    Ok(ApiResponse {
        message: "Upload URL generated. PUT your file to upload_url, then call /publish.".to_string(),
        error_type: ErrorType::Success,
        data: Some(data),
    })
}

#[derive(Deserialize)]
struct PublishQuery {
    version: Option<String>,
}

#[tracing::instrument(skip(state))]
async fn publish_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Query(query): Query<PublishQuery>,
) -> Result<ApiResponse<()>, AppError> {
    service::publish_plugin(state, claims, id, query.version).await?;
    Ok(ApiResponse {
        message: "Plugin version published successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

#[tracing::instrument(skip(state))]
async fn get_plugins(
    OptionalAuthUser(claims): OptionalAuthUser,
    State(state): State<SharedState>,
    ValidatedQuery(query): ValidatedQuery<PluginQuery>,
) -> Result<ApiResponse<PaginatedResponse<PluginResponse>>, AppError> {
    let data = service::get_plugins(state, claims, query).await?;
    Ok(ApiResponse {
        message: "Plugins retrieved successfully".to_string(),
        error_type: ErrorType::Success,
        data: Some(data),
    })
}

#[tracing::instrument(skip(state))]
async fn get_plugin(
    OptionalAuthUser(claims): OptionalAuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<PluginResponse>, AppError> {
    let data = service::get_plugin_by_id(state, claims, id).await?;
    Ok(ApiResponse {
        message: "Plugin retrieved successfully".to_string(),
        error_type: ErrorType::Success,
        data: Some(data),
    })
}

#[tracing::instrument(skip(state))]
async fn update_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    ValidatedJson(payload): ValidatedJson<UpdatePluginRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::update_plugin(state, claims, id, payload).await?;
    Ok(ApiResponse {
        message: "Plugin updated successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

#[tracing::instrument(skip(state))]
async fn delete_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    service::delete_plugin(state, claims, id).await?;
    Ok(ApiResponse {
        message: "Plugin deleted successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

#[tracing::instrument(skip(state))]
async fn vote_plugin(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    ValidatedJson(payload): ValidatedJson<VoteRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::vote_plugin(state, claims, id, payload).await?;
    Ok(ApiResponse {
        message: "Vote recorded successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

#[derive(Deserialize)]
struct DownloadQuery {
    version: Option<String>,
}

#[tracing::instrument(skip(state))]
async fn download_plugin(
    OptionalAuthUser(claims): OptionalAuthUser,
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Query(query): Query<DownloadQuery>,
) -> Result<ApiResponse<String>, AppError> {
    let url = service::download_plugin(state, claims, id, query.version).await?;
    Ok(ApiResponse {
        message: "Presigned download URL generated".to_string(),
        error_type: ErrorType::Success,
        data: Some(url),
    })
}

#[tracing::instrument(skip(state))]
async fn update_plugin_version(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path((id, version)): Path<(i64, String)>,
    ValidatedJson(payload): ValidatedJson<UpdatePluginVersionRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::update_plugin_version(state, claims, id, version, payload).await?;
    Ok(ApiResponse {
        message: "Plugin version updated successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

#[tracing::instrument(skip(state))]
async fn delete_plugin_version(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Path((id, version)): Path<(i64, String)>,
) -> Result<ApiResponse<()>, AppError> {
    service::delete_plugin_version(state, claims, id, version).await?;
    Ok(ApiResponse {
        message: "Plugin version deleted successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}
