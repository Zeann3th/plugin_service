use crate::SharedState;
use axum::{Router, extract::State, routing::get};

pub fn router() -> Router<SharedState> {
    Router::new().route("/profile", get(get_profile))
}

async fn get_profile(State(state): State<SharedState>) -> String {
    format!("Host: {}", state.config.service_host)
}
