use tower_http::cors::{CorsLayer};
use axum::http::{header, Method, HeaderValue};

pub fn cors_layer(allow_list: &str) -> CorsLayer {
    let origins: Vec<HeaderValue> = allow_list
        .split(',')
        .map(|s| s.trim().parse().expect("Invalid origin"))
        .collect();

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::PUT])
        .allow_origin(origins)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
