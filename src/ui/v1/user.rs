use crate::core::user::{
    model::{AuthResponse, SigninRequest, SignupRequest, UserInfo},
    service,
};
use crate::error::{ApiResponse, AppError, ErrorType};
use crate::state::SharedState;
use crate::ui::middlewares::auth::AuthUser;
use crate::ui::middlewares::validator::ValidatedJson;
use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/signin", post(signin))
        .route("/profile", get(get_profile))
}

async fn signup(
    State(state): State<SharedState>,
    ValidatedJson(payload): ValidatedJson<SignupRequest>,
) -> Result<ApiResponse<()>, AppError> {
    service::signup_user(state, payload).await?;

    Ok(ApiResponse {
        message: "User created successfully".to_string(),
        error_type: ErrorType::Success,
        data: None,
    })
}

async fn signin(
    State(state): State<SharedState>,
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<SigninRequest>,
) -> Result<(CookieJar, ApiResponse<AuthResponse>), AppError> {
    let (auth_res, refresh_cookie) = service::signin_user(state, payload).await?;

    let response = ApiResponse {
        message: "Login successful".to_string(),
        error_type: ErrorType::Success,
        data: Some(auth_res),
    };

    Ok((jar.add(refresh_cookie), response))
}

async fn get_profile(
    AuthUser(claims): AuthUser,
    State(_state): State<SharedState>,
) -> impl IntoResponse {
    ApiResponse {
        message: "Profile retrieved successfully".to_string(),
        error_type: ErrorType::Success,
        data: Some(UserInfo {
            id: claims.sub,
            username: claims.username,
            email: claims.email,
            role: claims.role,
            created_at: claims.created_at,
            updated_at: claims.updated_at,
        }),
    }
}
