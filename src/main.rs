use std::sync::Arc;

use axum::Router;

use crate::{
    config::{Config, load_config},
    database::PgConnectionPool,
};

mod config;
mod controllers;
mod database;

pub struct AppState {
    pub config: Config,
    pub db_pool: PgConnectionPool,
}

pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    let config: Config = load_config().expect("Failed to load config");

    let db_pool = database::connect(&config.database_url);

    let shared_state = Arc::new(AppState {
        config: config,
        db_pool: db_pool,
    });

    let app = Router::new()
        .nest("/plugins", controllers::plugin_controller::router())
        .nest("/users", controllers::user_controller::router())
        .with_state(Arc::clone(&shared_state));

    let addr = format!(
        "{}:{}",
        shared_state.config.service_host, shared_state.config.service_port
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}
