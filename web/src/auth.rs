use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{config::Config, error::AppError, state::AppState};

fn validate_token(token: &str, config: &Config) -> Result<(), AppError> {
    if token == config.api_key {
        Ok(())
    } else {
        Err(AppError::Unauthorized("Invalid API key".into()))
    }
}

pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Expects `Authorization: Bearer <API_KEY>`
    let token = headers
        .get("AUTHORIZATION")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".into()))?;

    validate_token(token, &state.config)?;

    Ok(next.run(req).await)
}
