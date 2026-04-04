use crate::schema::{plugins, plugin_versions, tags, plugin_tags, users, user_plugins};
use crate::{
    core::auth::jwt::{Claims, UserRole},
    error::AppError,
    state::SharedState,
};
use super::model::*;
use aws_sdk_s3::presigning::PresigningConfig;
use diesel::prelude::*;
use std::path::Path;
use std::time::Duration;

/// Step 1: Register plugin metadata. Returns plugin_id.
/// Creates the initial version with DRAFT status.
pub async fn create_plugin(
    state: SharedState,
    claims: Claims,
    payload: CreatePluginRequest,
) -> Result<CreatePluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    conn.transaction::<_, AppError, _>(|conn| {
        // Check if code is taken by another author
        let existing_plugin: Option<Plugin> = plugins::table
            .filter(plugins::code.eq(&payload.code))
            .first::<Plugin>(conn)
            .optional()
            .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;

        let plugin_id = if let Some(plugin) = existing_plugin {
            if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
                return Err(AppError::BadRequest(
                    "Plugin code is taken by another author".to_string(),
                ));
            }

            // Check if version already exists
            let existing_version = plugin_versions::table
                .filter(plugin_versions::plugin_id.eq(plugin.id))
                .filter(plugin_versions::version.eq(&payload.version))
                .first::<PluginVersion>(conn)
                .optional()
                .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;

            if existing_version.is_some() {
                return Err(AppError::BadRequest(
                    "This version already exists".to_string(),
                ));
            }
            plugin.id
        } else {
            let new_plugin = NewPlugin {
                code: payload.code.clone(),
                name: payload.name.clone(),
                description: payload.description.clone(),
                publisher_id: claims.sub,
            };

            let plugin: Plugin = diesel::insert_into(plugins::table)
                .values(&new_plugin)
                .get_result(conn)
                .map_err(|e| AppError::DatabaseError(format!("Failed to create plugin: {}", e)))?;
            plugin.id
        };

        // Create version
        let new_version = NewPluginVersion {
            plugin_id,
            version: payload.version.clone(),
            status: PluginStatus::Draft,
        };

        diesel::insert_into(plugin_versions::table)
            .values(&new_version)
            .execute(conn)
            .map_err(|e| AppError::DatabaseError(format!("Failed to create plugin version: {}", e)))?;

        // Handle tags
        if let Some(tag_names) = payload.tags {
            for tag_name in tag_names {
                let tag_name = tag_name.to_lowercase();
                
                // If not exists, create
                let tag: Tag = diesel::insert_into(tags::table)
                    .values(NewTag { name: tag_name.clone() })
                    .on_conflict(tags::name)
                    .do_update()
                    .set(tags::name.eq(tags::name)) // No-op to get the tag back
                    .get_result(conn)
                    .map_err(|e| AppError::DatabaseError(format!("Failed to ensure tag: {}", e)))?;

                let new_plugin_tag = NewPluginTag {
                    plugin_id,
                    tag_id: tag.id,
                };

                diesel::insert_into(plugin_tags::table)
                    .values(&new_plugin_tag)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .map_err(|e| AppError::DatabaseError(format!("Failed to link tag: {}", e)))?;
            }
        }

        Ok(CreatePluginResponse { plugin_id })
    })
}

/// Step 2: Generate a presigned S3 PUT URL for a specific version.
pub async fn get_upload_url(
    state: SharedState,
    claims: Claims,
    id: i64,
    payload: UploadPluginRequest,
) -> Result<UploadPluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to upload for this plugin".to_string(),
        ));
    }

    let version_str = payload.version.clone().unwrap_or_else(|| "1.0.0".to_string());

    let version = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(id))
        .filter(plugin_versions::version.eq(&version_str))
        .first::<PluginVersion>(&mut conn)
        .map_err(|_| AppError::NotFound(format!("Version {} not found", version_str)))?;

    let extension = Path::new(&payload.filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jar");

    // Naming convention: plugins/{code}/{version}/{code}-{version}.{ext}
    let key = format!(
        "plugins/{}/{}/{}-{}.{}",
        plugin.code, version.version, plugin.code, version.version, extension
    );

    // Persist file_path on the version
    diesel::update(plugin_versions::table.filter(plugin_versions::id.eq(version.id)))
        .set((
            plugin_versions::file_path.eq(Some(&key)),
            plugin_versions::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to update file path: {}", e)))?;

    let presigned_req = state
        .s3_client
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(&key)
        .content_length(payload.file_size)
        .presigned(
            PresigningConfig::expires_in(Duration::from_secs(3600))
                .unwrap(),
        )
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to generate presigned URL: {}", e))
        })?;

    let presigned_url = presigned_req.uri().to_string();
    let public_url = presigned_url.replace(
        &format!(
            "{}/{}",
            state.config.s3_endpoint, state.config.s3_bucket
        ),
        &state.config.s3_public_endpoint,
    );

    Ok(UploadPluginResponse {
        upload_url: public_url,
    })
}

/// Step 3: Mark a plugin version as PUBLISHED after a successful upload.
pub async fn publish_plugin(
    state: SharedState,
    claims: Claims,
    id: i64,
    version_str: Option<String>,
) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to publish this plugin".to_string(),
        ));
    }

    let mut query = plugin_versions::table.filter(plugin_versions::plugin_id.eq(id)).into_boxed();
    if let Some(v) = version_str {
        query = query.filter(plugin_versions::version.eq(v));
    } else {
        query = query.order_by(plugin_versions::created_at.desc());
    }

    let version = query.first::<PluginVersion>(&mut conn)
        .map_err(|_| AppError::NotFound("Version not found".to_string()))?;

    if version.file_path.is_none() {
        return Err(AppError::BadRequest(
            "No file uploaded yet for this version. Call /upload first.".to_string(),
        ));
    }

    diesel::update(plugin_versions::table.filter(plugin_versions::id.eq(version.id)))
        .set((
            plugin_versions::status.eq(PluginStatus::Published),
            plugin_versions::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to publish version: {}", e)))?;

    Ok(())
}

fn get_installation_status(
    latest_version: &Option<String>,
    user_version: &Option<String>,
) -> InstallationStatus {
    match (latest_version, user_version) {
        (Some(latest), Some(user)) => {
            if latest == user {
                InstallationStatus::Installed
            } else {
                // Simple string comparison for versions. 
                // In a real app, you might want to use a semver parser.
                InstallationStatus::Updatable
            }
        }
        _ => InstallationStatus::NotInstalled,
    }
}

pub async fn get_plugins(
    state: SharedState,
    claims: Option<Claims>,
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
    let mut count_query = plugins::table.inner_join(users::table).into_boxed();

    if let Some(ref code) = query.code {
        db_query = db_query.filter(plugins::code.ilike(format!("%{}%", code)));
        count_query = count_query.filter(plugins::code.ilike(format!("%{}%", code)));
    }

    if let Some(ref name) = query.name {
        db_query = db_query.filter(plugins::name.ilike(format!("%{}%", name)));
        count_query = count_query.filter(plugins::name.ilike(format!("%{}%", name)));
    }

    if let Some(ref tag) = query.tag {
        let tag_id_subquery = tags::table
            .filter(tags::name.eq(tag))
            .select(tags::id);
        
        let plugin_ids_subquery = plugin_tags::table
            .filter(plugin_tags::tag_id.eq_any(tag_id_subquery))
            .select(plugin_tags::plugin_id);
            
        db_query = db_query.filter(plugins::id.eq_any(plugin_ids_subquery));
        count_query = count_query.filter(plugins::id.eq_any(plugin_ids_subquery));
    }

    // Visibility filter for total count and query
    let requester_id = claims.as_ref().map(|c| c.sub).unwrap_or(0);
    let is_admin = claims.as_ref().map(|c| c.role == UserRole::Admin).unwrap_or(false);

    if !is_admin {
        // A plugin is visible if it has at least one published version OR the requester is the owner
        let published_subquery = plugin_versions::table
            .filter(plugin_versions::status.eq(PluginStatus::Published))
            .select(plugin_versions::plugin_id);

        db_query = db_query.filter(
            plugins::publisher_id.eq(requester_id)
            .or(plugins::id.eq_any(published_subquery.clone()))
        );
        count_query = count_query.filter(
            plugins::publisher_id.eq(requester_id)
            .or(plugins::id.eq_any(published_subquery))
        );
    }

    let total: i64 = count_query
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to count plugins: {}", e)))?;

    let plugins_raw: Vec<(Plugin, crate::core::user::model::User)> = db_query
        .select((Plugin::as_select(), crate::core::user::model::User::as_select()))
        .limit(per_page)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to load plugins: {}", e)))?;

    let mut items = Vec::new();

    for (p, u) in plugins_raw {
        tracing::debug!("Processing plugin: id={}, code={}", p.id, p.code);

        // Fetch tags
        let tags_raw = tags::table
            .inner_join(plugin_tags::table)
            .filter(plugin_tags::plugin_id.eq(p.id))
            .select(tags::name)
            .load::<String>(&mut conn)
            .map_err(|e| AppError::DatabaseError(format!("Failed to load tags: {}", e)))?;

        // Get latest version
        let latest_v: Option<String> = plugin_versions::table
            .filter(plugin_versions::plugin_id.eq(p.id))
            .filter(plugin_versions::status.eq(PluginStatus::Published))
            .order_by(plugin_versions::created_at.desc())
            .select(plugin_versions::version)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest version: {}", e)))?;

        // Visibility control: a plugin is visible if it has at least one published version,
        // or if the requester is the owner/admin.
        let is_owner = claims.as_ref().map(|c| c.sub == p.publisher_id).unwrap_or(false);
        let is_admin = claims.as_ref().map(|c| c.role == UserRole::Admin).unwrap_or(false);

        if latest_v.is_none() && !is_owner && !is_admin {
            tracing::info!("Skipping plugin {} because it has no published versions and requester is not owner/admin", p.code);
            continue;
        }

        // Check user installation status
        let mut user_v: Option<String> = None;
        if let Some(ref c) = claims {
            user_v = user_plugins::table
                .filter(user_plugins::user_id.eq(c.sub))
                .filter(user_plugins::plugin_id.eq(p.id))
                .select(user_plugins::version)
                .first::<String>(&mut conn)
                .optional()
                .map_err(|e| AppError::DatabaseError(format!("Failed to fetch user plugin: {}", e)))?;
        }

        tracing::debug!("Plugin {} added to response (latest_v={:?}, user_v={:?}, is_owner={})", p.code, latest_v, user_v, is_owner);

        items.push(PluginResponse {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            publisher: UserInfo {
                id: u.id,
                username: u.username,
            },
            upvote_count: p.upvote_count.unwrap_or(0),
            downvote_count: p.downvote_count.unwrap_or(0),
            tags: tags_raw,
            latest_version: latest_v.clone(),
            installation_status: get_installation_status(&latest_v, &user_v),
            versions: None,
            created_at: p.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            updated_at: p.updated_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        });
    }

    Ok(PaginatedResponse {
        items,
        total,
        page,
        per_page,
    })
}

pub async fn get_plugin_by_id(state: SharedState, claims: Option<Claims>, id: i64) -> Result<PluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let (p, u): (Plugin, crate::core::user::model::User) = plugins::table
        .inner_join(users::table)
        .filter(plugins::id.eq(id))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    // Get latest version
    let latest_v: Option<String> = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(p.id))
        .filter(plugin_versions::status.eq(PluginStatus::Published))
        .order_by(plugin_versions::created_at.desc())
        .select(plugin_versions::version)
        .first::<String>(&mut conn)
        .optional()
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest version: {}", e)))?;

    // Check user installation status
    let mut user_v: Option<String> = None;
    if let Some(ref c) = claims {
        user_v = user_plugins::table
            .filter(user_plugins::user_id.eq(c.sub))
            .filter(user_plugins::plugin_id.eq(p.id))
            .select(user_plugins::version)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch user plugin: {}", e)))?;
    }

    // Fetch versions
    let mut v_query = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(p.id))
        .into_boxed();
    
    let is_owner = claims.as_ref().map(|c| c.sub == p.publisher_id).unwrap_or(false);
    let is_admin = claims.as_ref().map(|c| c.role == UserRole::Admin).unwrap_or(false);
    
    if !is_owner && !is_admin {
        v_query = v_query.filter(plugin_versions::status.eq(PluginStatus::Published));
    }

    let versions_raw = v_query
        .order_by(plugin_versions::created_at.desc())
        .load::<PluginVersion>(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to load versions: {}", e)))?;

    let versions = versions_raw.into_iter().map(|v| PluginVersionResponse {
        version: v.version,
        status: v.status,
        download_count: v.download_count.unwrap_or(0),
        created_at: v.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
    }).collect();

    let tags_raw = tags::table
        .inner_join(plugin_tags::table)
        .filter(plugin_tags::plugin_id.eq(p.id))
        .select(tags::name)
        .load::<String>(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to load tags: {}", e)))?;

    Ok(PluginResponse {
        id: p.id,
        code: p.code,
        name: p.name,
        description: p.description,
        publisher: UserInfo {
            id: u.id,
            username: u.username,
        },
        upvote_count: p.upvote_count.unwrap_or(0),
        downvote_count: p.downvote_count.unwrap_or(0),
        tags: tags_raw,
        latest_version: latest_v.clone(),
        installation_status: get_installation_status(&latest_v, &user_v),
        versions: Some(versions),
        created_at: p.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        updated_at: p.updated_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
    })
}

pub async fn update_plugin(
    state: SharedState,
    claims: Claims,
    id: i64,
    payload: UpdatePluginRequest,
) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to update this plugin".to_string(),
        ));
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

pub async fn delete_plugin(state: SharedState, claims: Claims, id: i64) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to delete this plugin".to_string(),
        ));
    }

    diesel::delete(plugins::table.filter(plugins::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete plugin: {}", e)))?;

    Ok(())
}

pub async fn vote_plugin(
    state: SharedState,
    _claims: Claims,
    id: i64,
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
    }
    .map_err(|e| AppError::DatabaseError(format!("Failed to vote: {}", e)))?;

    Ok(())
}

/// Download: uses stored file_path on version.
/// Records download in user_plugins if authenticated.
pub async fn download_plugin(
    state: SharedState,
    claims: Option<Claims>,
    id: i64,
    version_str: Option<String>
) -> Result<String, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let mut query = plugin_versions::table.filter(plugin_versions::plugin_id.eq(id)).into_boxed();
    
    if let Some(v) = version_str {
        query = query.filter(plugin_versions::version.eq(v));
    } else {
        query = query.filter(plugin_versions::status.eq(PluginStatus::Published))
                     .order_by(plugin_versions::created_at.desc());
    }

    let version = query.first::<PluginVersion>(&mut conn)
        .map_err(|_| AppError::NotFound("Version not found or not published".to_string()))?;

    if version.status != PluginStatus::Published {
        return Err(AppError::BadRequest(
            "Plugin version is not published yet".to_string(),
        ));
    }

    let key = version
        .file_path
        .ok_or_else(|| AppError::BadRequest("No file available for this version".to_string()))?;

    // Increment download count
    diesel::update(plugin_versions::table.filter(plugin_versions::id.eq(version.id)))
        .set(plugin_versions::download_count.eq(plugin_versions::download_count + 1))
        .execute(&mut conn)
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to increment download count: {}", e))
        })?;

    // Record user download if authenticated
    if let Some(c) = claims {
        let new_user_plugin = NewUserPlugin {
            user_id: c.sub,
            plugin_id: id,
            version: version.version.clone(),
        };

        diesel::insert_into(user_plugins::table)
            .values(&new_user_plugin)
            .on_conflict((user_plugins::user_id, user_plugins::plugin_id))
            .do_update()
            .set(user_plugins::version.eq(version.version.clone()))
            .execute(&mut conn)
            .map_err(|e| AppError::DatabaseError(format!("Failed to record download: {}", e)))?;
    }

    let presigned_req = state
        .s3_client
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(key)
        .presigned(
            PresigningConfig::expires_in(Duration::from_secs(3600))
                .unwrap(),
        )
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to generate presigned URL: {}", e))
        })?;

    let presigned_url = presigned_req.uri().to_string();
    let public_url = presigned_url.replace(
        &format!(
            "{}/{}",
            state.config.s3_endpoint, state.config.s3_bucket
        ),
        &state.config.s3_public_endpoint,
    );

    Ok(public_url)
}
