use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{
    error::{AppError, Result},
    handlers::objects::AppState,
};

pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(t) if t == state.auth_token => {
            tracing::debug!("Authentication successful");
            Ok(next.run(request).await)
        }
        Some(_) => {
            tracing::warn!("Authentication failed: invalid token");
            Err(AppError::Unauthorized)
        }
        None => {
            tracing::warn!("Authentication failed: no token provided");
            Err(AppError::Unauthorized)
        }
    }
}
