use tower_http::cors::{Any, CorsLayer};
use axum::http::{header, Method};

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::PUT])
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
