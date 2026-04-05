// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "plugin_status"))]
    pub struct PluginStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_role"))]
    pub struct UserRole;
}

diesel::table! {
    plugin_tags (plugin_id, tag_id) {
        plugin_id -> Int8,
        tag_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PluginStatus;

    plugin_versions (id) {
        id -> Int8,
        plugin_id -> Int8,
        #[max_length = 50]
        version -> Varchar,
        #[max_length = 500]
        file_path -> Nullable<Varchar>,
        download_count -> Nullable<Int4>,
        status -> PluginStatus,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PluginStatus;

    plugins (id) {
        id -> Int8,
        #[max_length = 255]
        code -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        publisher_id -> Int8,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        upvote_count -> Nullable<Int4>,
        downvote_count -> Nullable<Int4>,
        status -> PluginStatus,
        #[max_length = 255]
        github_repo -> Nullable<Varchar>,
    }
}

diesel::table! {
    tags (id) {
        id -> Int8,
        #[max_length = 50]
        name -> Varchar,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    user_plugins (id) {
        id -> Int8,
        user_id -> Int8,
        plugin_id -> Int8,
        #[max_length = 50]
        version -> Varchar,
        downloaded_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserRole;

    users (id) {
        id -> Int8,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        role -> UserRole,
    }
}

diesel::joinable!(plugin_tags -> plugins (plugin_id));
diesel::joinable!(plugin_tags -> tags (tag_id));
diesel::joinable!(plugin_versions -> plugins (plugin_id));
diesel::joinable!(plugins -> users (publisher_id));
diesel::joinable!(user_plugins -> plugins (plugin_id));
diesel::joinable!(user_plugins -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    plugin_tags,
    plugin_versions,
    plugins,
    tags,
    user_plugins,
    users,
);
