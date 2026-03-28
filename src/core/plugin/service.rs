use crate::schema::{plugins, users};
use crate::{
    core::auth::jwt::{UserRole, Claims},
    error::AppError,
    state::SharedState,
};
use super::model::*;
use aws_sdk_s3::presigning::PresigningConfig;
use diesel::prelude::*;
use std::time::Duration;
use std::path::Path;

pub async fn create_plugin(
    state: SharedState,
    claims: Claims,
    payload: CreatePluginRequest,
) -> Result<CreatePluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    // Check if plugin with same code and version exists
    let existing_plugin = plugins::table
        .filter(plugins::code.eq(&payload.code))
        .filter(plugins::version.eq(&payload.version))
        .first::<Plugin>(&mut conn)
        .optional()
        .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;

    if let Some(plugin) = existing_plugin {
        if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
            return Err(AppError::BadRequest("Plugin code already exists and you are not the author".to_string()));
        }
        return Err(AppError::BadRequest("This version already exists".to_string()));
    }

    // Check if code exists with DIFFERENT version
    let any_version = plugins::table
        .filter(plugins::code.eq(&payload.code))
        .first::<Plugin>(&mut conn)
        .optional()
        .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;

    if let Some(plugin) = any_version {
         if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
            return Err(AppError::BadRequest("Plugin code is taken by another author".to_string()));
        }
    }

    let new_plugin = NewPlugin {
        code: payload.code.clone(),
        name: payload.name,
        description: payload.description,
        version: payload.version.clone(),
        publisher_id: claims.sub,
    };

    let plugin: Plugin = diesel::insert_into(plugins::table)
        .values(&new_plugin)
        .get_result(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to create plugin: {}", e)))?;

    // Naming convention: {plugin-code}-{version}.{extension}
    let extension = Path::new(&payload.filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jar");
    
    let key = format!(
        "plugins/{}/{}/{}-{}.{}",
        payload.code, payload.version, payload.code, payload.version, extension
    );

    let presigned_req = state.s3_client
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate presigned URL: {}", e)))?;

    Ok(CreatePluginResponse {
        plugin_id: plugin.id,
        upload_url: presigned_req.uri().to_string(),
    })
}

pub async fn get_plugins(
    state: SharedState,
    query: PluginQuery,
) -> Result<PaginatedResponse<PluginResponse>, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let offset = (page - 1) * per_page;

    let mut db_query = plugins::table.inner_join(users::table).into_boxed();

    if let Some(ref code) = query.code {
        db_query = db_query.filter(plugins::code.ilike(format!("%{}%", code)));
    }

    if let Some(ref name) = query.name {
        db_query = db_query.filter(plugins::name.ilike(format!("%{}%", name)));
    }

    let items_raw: Vec<(Plugin, crate::core::user::model::User)> = db_query
        .limit(per_page)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to load plugins: {}", e)))?;

    let mut count_query = plugins::table.inner_join(users::table).into_boxed();
    if let Some(code_val) = query.code {
        count_query = count_query.filter(plugins::code.ilike(format!("%{}%", code_val)));
    }
    if let Some(name_val) = query.name {
        count_query = count_query.filter(plugins::name.ilike(format!("%{}%", name_val)));
    }
    let total: i64 = count_query.count().get_result(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to count plugins: {}", e)))?;

    let items = items_raw.into_iter().map(|(p, u)| PluginResponse {
        id: p.id,
        code: p.code,
        name: p.name,
        description: p.description,
        version: p.version,
        publisher: UserInfo { id: u.id, username: u.username },
        download_count: p.download_count.unwrap_or(0),
        upvote_count: p.upvote_count.unwrap_or(0),
        downvote_count: p.downvote_count.unwrap_or(0),
        created_at: p.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        updated_at: p.updated_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
    }).collect();

    Ok(PaginatedResponse {
        items,
        total,
        page,
        per_page,
    })
}

pub async fn get_plugin_by_id(state: SharedState, id: i32) -> Result<PluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let (p, u): (Plugin, crate::core::user::model::User) = plugins::table
        .inner_join(users::table)
        .filter(plugins::id.eq(id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    Ok(PluginResponse {
        id: p.id,
        code: p.code,
        name: p.name,
        description: p.description,
        version: p.version,
        publisher: UserInfo { id: u.id, username: u.username },
        download_count: p.download_count.unwrap_or(0),
        upvote_count: p.upvote_count.unwrap_or(0),
        downvote_count: p.downvote_count.unwrap_or(0),
        created_at: p.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        updated_at: p.updated_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
    })
}

pub async fn update_plugin(
    state: SharedState,
    claims: Claims,
    id: i32,
    payload: UpdatePluginRequest,
) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table.filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::BadRequest("Not authorized to update this plugin".to_string()));
    }

    diesel::update(plugins::table.filter(plugins::id.eq(id)))
        .set((
            payload.name.map(|n| plugins::name.eq(n)),
            payload.description.map(|d| plugins::description.eq(d)),
            plugins::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to update plugin: {}", e)))?;

    Ok(())
}

pub async fn delete_plugin(state: SharedState, claims: Claims, id: i32) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table.filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::BadRequest("Not authorized to delete this plugin".to_string()));
    }

    diesel::delete(plugins::table.filter(plugins::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete plugin: {}", e)))?;

    Ok(())
}

pub async fn vote_plugin(
    state: SharedState,
    _claims: Claims,
    id: i32,
    payload: VoteRequest,
) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    if payload.is_upvote {
        diesel::update(plugins::table.filter(plugins::id.eq(id)))
            .set(plugins::upvote_count.eq(plugins::upvote_count + 1))
            .execute(&mut conn)
    } else {
        diesel::update(plugins::table.filter(plugins::id.eq(id)))
            .set(plugins::downvote_count.eq(plugins::downvote_count + 1))
            .execute(&mut conn)
    }.map_err(|e| AppError::DatabaseError(format!("Failed to vote: {}", e)))?;

    Ok(())
}

pub async fn download_plugin(state: SharedState, id: i32, filename: String) -> Result<String, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table.filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    // Inc download counter
    diesel::update(plugins::table.filter(plugins::id.eq(id)))
        .set(plugins::download_count.eq(plugins::download_count + 1))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to increment download count: {}", e)))?;

    // Use the same convention for finding the file
    let extension = Path::new(&filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jar");

    let key = format!(
        "plugins/{}/{}/{}-{}.{}",
        plugin.code, plugin.version, plugin.code, plugin.version, extension
    );

    let presigned_req = state.s3_client
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate presigned URL: {}", e)))?;

    Ok(presigned_req.uri().to_string())
}
