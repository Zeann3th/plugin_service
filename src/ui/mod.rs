pub mod v1;
pub mod health;
pub mod middlewares;

use axum::Router;
use crate::state::SharedState;

pub fn create_router() -> Router<SharedState> {
    Router::new()
        .nest("/api/v1/users", v1::user::router())
        .nest("/api/v1/plugins", v1::plugin::router())
        .nest("/api", health::router())
}
