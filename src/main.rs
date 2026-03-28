use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;

mod config;
mod core;
mod error;
mod infrastructure;
mod schema;
mod state;
mod ui;

use crate::{
    config::load_config,
    state::{AppState, SharedState},
    ui::middlewares::{cors, wrapper},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = load_config().expect("Failed to load config");

    let db_pool = infrastructure::database::connect(&config.database_url);
    let s3_client = infrastructure::s3::connect(&config).await;

    // Ensure bucket exists
    infrastructure::s3::ensure_bucket_exists(&s3_client, &config.s3_bucket).await;

    let shared_state: SharedState = Arc::new(AppState {
        config: config,
        db_pool: db_pool,
        s3_client: s3_client,
    });

    let app = ui::create_router()
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CatchPanicLayer::custom(wrapper::global_panic_handler))
                .layer(cors::cors_layer()),
        )
        .with_state(Arc::clone(&shared_state));

    let addr = format!(
        "{}:{}",
        shared_state.config.service_host, shared_state.config.service_port
    );

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to address {}: {}", addr, e));

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .await
        .expect("Failed to run server");
}
