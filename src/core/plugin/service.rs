use crate::schema::{plugins, plugin_versions, tags, plugin_tags, users, user_plugins};
use crate::{
    core::auth::jwt::{Claims, UserRole},
    error::AppError,
    state::SharedState,
};
use super::model::*;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Step 1: Register plugin metadata. Returns plugin_id.
/// Creates the initial version with DRAFT status.
#[tracing::instrument(skip(state))]
pub async fn create_plugin(
    state: SharedState,
    claims: Claims,
    payload: CreatePluginRequest,
) -> Result<CreatePluginResponse, AppError> {
    tracing::info!("Creating new plugin: {} ({})", payload.name, payload.code);
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    conn.transaction::<_, AppError, _>(|conn| {
        // Check if code is taken by another author
        let existing_plugin: Option<Plugin> = plugins::table
            .filter(plugins::code.eq(&payload.code))
            .select(Plugin::as_select())
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
                .select(PluginVersion::as_select())
                .first::<PluginVersion>(conn)                .optional()
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
                github_repo: payload.github_repo.clone(),
                publisher_id: claims.sub,
                status: PluginStatus::Draft,
            };

            let plugin: Plugin = diesel::insert_into(plugins::table)
                .values(&new_plugin)
                .returning(Plugin::as_select())
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
#[tracing::instrument(skip(state))]
pub async fn get_upload_url(
    state: SharedState,
    claims: Claims,
    id: i64,
    version_name: String,
    payload: UploadPluginRequest,
) -> Result<UploadPluginResponse, AppError> {
    tracing::info!("Generating upload URL for plugin id: {}, version: {}", id, version_name);
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .select(Plugin::as_select())
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to upload for this plugin".to_string(),
        ));
    }

    let version = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(id))
        .filter(plugin_versions::version.eq(&version_name))
        .first::<PluginVersion>(&mut conn)
        .map_err(|_| AppError::NotFound(format!("Version {} not found", version_name)))?;

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

    // Use standard s3_client which signs against s3_endpoint (internal R2 endpoint)
    // The browser will upload directly to this URL.
    let presigned_req = state
        .s3_client
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(&key)
        .content_length(payload.file_size)
        .presigned(
            aws_sdk_s3::presigning::PresigningConfig::expires_in(Duration::from_secs(3600))
                .unwrap(),
        )
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to generate presigned URL: {}", e))
        })?;

    Ok(UploadPluginResponse {
        upload_url: presigned_req.uri().to_string(),
    })
}

/// Step 3: Mark a plugin version as PUBLISHED after a successful upload.
#[tracing::instrument(skip(state))]
pub async fn publish_plugin(
    state: SharedState,
    claims: Claims,
    id: i64,
    version_name: String,
) -> Result<(), AppError> {
    tracing::info!("Publishing plugin id: {}, version: {}", id, version_name);
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let plugin = plugins::table
        .filter(plugins::id.eq(id))
        .select(Plugin::as_select())
        .first::<Plugin>(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Not authorized to publish this plugin".to_string(),
        ));
    }

    let version = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(id))
        .filter(plugin_versions::version.eq(&version_name))
        .first::<PluginVersion>(&mut conn)
        .map_err(|_| AppError::NotFound(format!("Version {} not found", version_name)))?;

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
                InstallationStatus::Updatable
            }
        }
        _ => InstallationStatus::NotInstalled,
    }
}

#[tracing::instrument(skip(state))]
pub async fn get_plugins(
    state: SharedState,
    claims: Option<Claims>,
    query: PluginQuery,
) -> Result<PaginatedResponse<PluginResponse>, AppError> {
    tracing::debug!("Fetching plugins with query: {:?}", query);
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

    let requester_id = claims.as_ref().map(|c| c.sub).unwrap_or(0);
    let is_admin = claims.as_ref().map(|c| c.role == UserRole::Admin).unwrap_or(false);

    if !is_admin {
        let published_subquery = plugin_versions::table
            .filter(plugin_versions::status.eq(PluginStatus::Published))
            .select(plugin_versions::plugin_id);

        db_query = db_query.filter(
            plugins::publisher_id.eq(requester_id)
            .or(
                plugins::status.eq(PluginStatus::Published)
                .and(plugins::id.eq_any(published_subquery.clone()))
            )
        );
        count_query = count_query.filter(
            plugins::publisher_id.eq(requester_id)
            .or(
                plugins::status.eq(PluginStatus::Published)
                .and(plugins::id.eq_any(published_subquery))
            )
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

    if plugins_raw.is_empty() {
        return Ok(PaginatedResponse {
            items: Vec::new(),
            total,
            page,
            per_page,
        });
    }

    let plugin_ids: Vec<i64> = plugins_raw.iter().map(|(p, _)| p.id).collect();

    let all_tags_raw: Vec<(i64, String)> = plugin_tags::table
        .inner_join(tags::table)
        .filter(plugin_tags::plugin_id.eq_any(&plugin_ids))
        .select((plugin_tags::plugin_id, tags::name))
        .load(&mut conn)
        .map_err(|e| AppError::DatabaseError(format!("Failed to load tags: {}", e)))?;

    let mut tags_map: HashMap<i64, Vec<String>> = HashMap::new();
    for (pid, tname) in all_tags_raw {
        tags_map.entry(pid).or_default().push(tname);
    }

    let latest_versions_raw: Vec<PluginVersion> = if is_admin {
        plugin_versions::table
            .filter(plugin_versions::plugin_id.eq_any(&plugin_ids))
            .order_by((plugin_versions::plugin_id, plugin_versions::created_at.desc()))
            .distinct_on(plugin_versions::plugin_id)
            .select(PluginVersion::as_select())
            .load(&mut conn)
    } else {
        plugin_versions::table
            .filter(plugin_versions::plugin_id.eq_any(&plugin_ids))
            .filter(
                plugin_versions::status.eq(PluginStatus::Published)
                .or(plugin_versions::plugin_id.eq_any(
                    plugins::table
                        .filter(plugins::id.eq_any(&plugin_ids))
                        .filter(plugins::publisher_id.eq(requester_id))
                        .select(plugins::id)
                ))
            )
            .order_by((plugin_versions::plugin_id, plugin_versions::created_at.desc()))
            .distinct_on(plugin_versions::plugin_id)
            .select(PluginVersion::as_select())
            .load(&mut conn)
    }.map_err(|e| AppError::DatabaseError(format!("Failed to load latest versions: {}", e)))?;

    let mut latest_versions_map: HashMap<i64, String> = HashMap::new();
    for v in latest_versions_raw {
        latest_versions_map.insert(v.plugin_id, v.version);
    }

    let mut user_versions_map: HashMap<i64, String> = HashMap::new();
    if let Some(ref c) = claims {
        let user_plugins_raw: Vec<(i64, String)> = user_plugins::table
            .filter(user_plugins::user_id.eq(c.sub))
            .filter(user_plugins::plugin_id.eq_any(&plugin_ids))
            .select((user_plugins::plugin_id, user_plugins::version))
            .load(&mut conn)
            .map_err(|e| AppError::DatabaseError(format!("Failed to load user installations: {}", e)))?;
        
        for (pid, v) in user_plugins_raw {
            user_versions_map.insert(pid, v);
        }
    }

    let items = plugins_raw.into_iter().filter_map(|(p, u)| {
        let latest_v = latest_versions_map.get(&p.id).cloned();
        let is_owner = requester_id == p.publisher_id;
        if latest_v.is_none() && !is_owner && !is_admin {
            return None;
        }

        let user_v = user_versions_map.get(&p.id).cloned();
        let tags = tags_map.remove(&p.id).unwrap_or_default();

        Some(PluginResponse {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            github_repo: p.github_repo,
            status: p.status,
            publisher: UserInfo {
                id: u.id,
                username: u.username,
            },
            upvote_count: p.upvote_count.unwrap_or(0),
            downvote_count: p.downvote_count.unwrap_or(0),
            tags,
            latest_version: latest_v.clone(),
            installation_status: get_installation_status(&latest_v, &user_v),
            versions: None,
            created_at: p.created_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            updated_at: p.updated_at.unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        })
    }).collect();

    Ok(PaginatedResponse {
        items,
        total,
        page,
        per_page,
    })
}

#[tracing::instrument(skip(state))]
pub async fn get_plugin_by_id(state: SharedState, claims: Option<Claims>, id: i64) -> Result<PluginResponse, AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    let (p, u): (Plugin, crate::core::user::model::User) = plugins::table
        .inner_join(users::table)
        .filter(plugins::id.eq(id))
        .select((Plugin::as_select(), crate::core::user::model::User::as_select()))
        .first(&mut conn)
        .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

    let latest_v: Option<String> = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(p.id))
        .filter(plugin_versions::status.eq(PluginStatus::Published))
        .order_by(plugin_versions::created_at.desc())
        .select(plugin_versions::version)
        .first::<String>(&mut conn)
        .optional()
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest version: {}", e)))?;

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

    let mut v_query = plugin_versions::table
        .filter(plugin_versions::plugin_id.eq(p.id))
        .into_boxed();
    
    let is_owner = claims.as_ref().map(|c| c.sub == p.publisher_id).unwrap_or(false);
    let is_admin = claims.as_ref().map(|c| c.role == UserRole::Admin).unwrap_or(false);
    
    if !is_owner && !is_admin {
        v_query = v_query.filter(plugin_versions::status.eq(PluginStatus::Published));
    }

    let versions_raw = v_query
        .select(PluginVersion::as_select())
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
        github_repo: p.github_repo,
        status: p.status,
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

#[tracing::instrument(skip(state))]
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

    conn.transaction::<_, AppError, _>(|conn| {
        let plugin = plugins::table
            .filter(plugins::id.eq(id))
            .select(Plugin::as_select())
            .first::<Plugin>(conn)
            .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

        if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
            return Err(AppError::Forbidden("Not authorized".to_string()));
        }

        diesel::update(plugins::table.filter(plugins::id.eq(id)))
            .set((
                payload.name.as_ref().map(|n| plugins::name.eq(n)),
                payload.description.as_ref().map(|d| plugins::description.eq(d)),
                payload.github_repo.as_ref().map(|g| plugins::github_repo.eq(g)),
                payload.status.as_ref().map(|s| plugins::status.eq(s)),
                plugins::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
            .map_err(|e| AppError::DatabaseError(format!("Failed to update plugin: {}", e)))?;

        if let Some(tag_names) = payload.tags {
            diesel::delete(plugin_tags::table.filter(plugin_tags::plugin_id.eq(id)))
                .execute(conn)
                .map_err(|e| AppError::DatabaseError(format!("Failed to clear old tags: {}", e)))?;

            for tag_name in tag_names {
                let tag_name = tag_name.to_lowercase();
                let tag: Tag = diesel::insert_into(tags::table)
                    .values(NewTag { name: tag_name.clone() })
                    .on_conflict(tags::name).do_update().set(tags::name.eq(tags::name)).get_result(conn)
                    .map_err(|e| AppError::DatabaseError(format!("Failed to ensure tag: {}", e)))?;

                diesel::insert_into(plugin_tags::table).values(NewPluginTag { plugin_id: id, tag_id: tag.id }).on_conflict_do_nothing().execute(conn)
                    .map_err(|e| AppError::DatabaseError(format!("Failed to link tag: {}", e)))?;
            }
            cleanup_orphan_tags(conn)?;
        }
        Ok(())
    })
}

#[tracing::instrument(skip(state))]
pub async fn update_plugin_version(
    state: SharedState,
    claims: Claims,
    id: i64,
    version_str: String,
    payload: UpdatePluginVersionRequest,
) -> Result<(), AppError> {
    let mut conn = state
        .db_pool
        .get()
        .map_err(|e| AppError::DatabaseError(format!("Failed to get DB connection: {}", e)))?;

    conn.transaction::<_, AppError, _>(|conn| {
        let plugin = plugins::table.filter(plugins::id.eq(id)).select(Plugin::as_select()).first::<Plugin>(conn)
            .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;

        if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin {
            return Err(AppError::Forbidden("Not authorized".to_string()));
        }

        let updated_count = diesel::update(plugin_versions::table.filter(plugin_versions::plugin_id.eq(id)).filter(plugin_versions::version.eq(version_str)))
        .set((payload.status.map(|s| plugin_versions::status.eq(s)), plugin_versions::updated_at.eq(chrono::Utc::now().naive_utc())))
        .execute(conn).map_err(|e| AppError::DatabaseError(format!("Failed to update version: {}", e)))?;

        if updated_count == 0 { return Err(AppError::NotFound("Version not found".to_string())); }
        Ok(())
    })
}

#[tracing::instrument(skip(state))]
pub async fn delete_plugin(state: SharedState, claims: Claims, id: i64) -> Result<(), AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::DatabaseError(format!("Failed to connect: {}", e)))?;
    conn.transaction::<_, AppError, _>(|conn| {
        let plugin = plugins::table.filter(plugins::id.eq(id)).select(Plugin::as_select()).first::<Plugin>(conn)
            .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;
        if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin { return Err(AppError::Forbidden("Not authorized".to_string())); }
        diesel::delete(plugins::table.filter(plugins::id.eq(id))).execute(conn).map_err(|e| AppError::DatabaseError(format!("Delete failed: {}", e)))?;
        cleanup_orphan_tags(conn)?;
        Ok(())
    })
}

#[tracing::instrument(skip(state))]
pub async fn delete_plugin_version(state: SharedState, claims: Claims, id: i64, version: String) -> Result<(), AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::DatabaseError(format!("Failed to connect: {}", e)))?;
    conn.transaction::<_, AppError, _>(|conn| {
        let plugin = plugins::table.filter(plugins::id.eq(id)).select(Plugin::as_select()).first::<Plugin>(conn)
            .map_err(|_| AppError::NotFound("Plugin not found".to_string()))?;
        if plugin.publisher_id != claims.sub && claims.role != UserRole::Admin { return Err(AppError::Forbidden("Not authorized".to_string())); }
        let deleted_count = diesel::delete(plugin_versions::table.filter(plugin_versions::plugin_id.eq(id)).filter(plugin_versions::version.eq(version)))
        .execute(conn).map_err(|e| AppError::DatabaseError(format!("Delete failed: {}", e)))?;
        if deleted_count == 0 { return Err(AppError::NotFound("Version not found".to_string())); }
        Ok(())
    })
}

fn cleanup_orphan_tags(conn: &mut PgConnection) -> Result<(), AppError> {
    diesel::delete(tags::table.filter(diesel::dsl::not(diesel::dsl::exists(plugin_tags::table.filter(plugin_tags::tag_id.eq(tags::id))))))
    .execute(conn).map_err(|e| AppError::DatabaseError(format!("Cleanup failed: {}", e)))?;
    Ok(())
}

#[tracing::instrument(skip(state))]
pub async fn vote_plugin(state: SharedState, _claims: Claims, id: i64, payload: VoteRequest) -> Result<(), AppError> {
    let mut conn = state.db_pool.get().map_err(|e| AppError::DatabaseError(format!("Connect failed: {}", e)))?;
    if payload.is_upvote { diesel::update(plugins::table.filter(plugins::id.eq(id))).set(plugins::upvote_count.eq(plugins::upvote_count + 1)).execute(&mut conn) }
    else { diesel::update(plugins::table.filter(plugins::id.eq(id))).set(plugins::downvote_count.eq(plugins::downvote_count + 1)).execute(&mut conn) }
    .map_err(|e| AppError::DatabaseError(format!("Vote failed: {}", e)))?;
    Ok(())
}
