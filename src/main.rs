use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::catch_panic::CatchPanicLayer;

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
    ui::middlewares::{cors, helmet, logger, wrapper},
};

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "plugin_service=debug,tower_http=debug".into()),
        )
        .init();
}

async fn init_state(config: crate::config::Config) -> SharedState {
    let db_pool = infrastructure::database::connect(&config.database_url);
    tracing::info!("Database connected");

    let s3_client = infrastructure::s3::connect(&config).await;
    infrastructure::s3::ensure_bucket_exists(&s3_client, &config.s3_bucket).await;
    tracing::info!("S3 connected");

    Arc::new(AppState {
        config,
        db_pool,
        s3_client,
    })
}

#[tokio::main]
async fn main() {
    init_tracing();
    tracing::info!("Starting plugin_service...");

    let config = load_config().expect("Failed to load config");
    tracing::info!("Config loaded");

    let addr = format!("{}:{}", config.service_host, config.service_port);
    let cors_allow_list = config.cors_allow_list.clone();
    let shared_state = init_state(config).await;

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(20)
            .finish()
            .unwrap(),
    );

    let limiter = governor_conf.limiter().clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            limiter.retain_recent();
            tracing::info!("rate limiter storage size: {}", limiter.len());
        }
    });

    let app = ui::create_router()
        .layer(
            ServiceBuilder::new()
                .layer(logger::logger_layer())
                .layer(CatchPanicLayer::custom(wrapper::global_panic_handler))
                .layer(GovernorLayer::new(governor_conf))
                .layer(helmet::helmet_layer())
                .layer(cors::cors_layer(&cors_allow_list)),
        )
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", addr, e));

    tracing::info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| panic!("Server error: {}", e));

    tracing::info!("Server stopped.");
}
