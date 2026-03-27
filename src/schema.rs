// @generated automatically by Diesel CLI.

diesel::table! {
    plugins (id) {
        id -> Int4,
        #[max_length = 255]
        code -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        #[max_length = 50]
        version -> Varchar,
        publisher_id -> Int8,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        download_count -> Nullable<Int4>,
        upvote_count -> Nullable<Int4>,
        downvote_count -> Nullable<Int4>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(plugins -> users (publisher_id));

diesel::allow_tables_to_appear_in_same_query!(plugins, users,);
