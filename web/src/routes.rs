use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;

use crate::{
    error::AppError,
    models::{BeatRequest, StatusResponse},
    services::{self, BeatAction},
    state::AppState,
};

pub async fn beat(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BeatRequest>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(?body, "received beat");

    let now = Utc::now();

    // Start a transaction
    let mut tx = state.db.begin().await?;

    let latest = services::fetch_latest_period(&mut tx, &body.device_id).await?;

    match services::decide_beat_action(latest.as_ref(), body.boot_id, body.uptime_ms, now) {
        BeatAction::Insert { started_at } => {
            services::insert_power_period(
                &mut tx,
                &body.device_id,
                body.boot_id,
                body.reset_reason,
                started_at,
                now,
            )
            .await?
        }
        BeatAction::Update { id } => services::touch_power_period(&mut tx, id, now).await?,
    }

    tx.commit().await?;

    // Beat accepted, nothing to return
    Ok(StatusCode::NO_CONTENT)
}

pub async fn status(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<String>,
) -> Result<Json<StatusResponse>, AppError> {
    let mut conn = state.db.acquire().await?;

    let Some(latest) = services::fetch_latest_period(&mut conn, &device_id).await? else {
        return Err(AppError::NotFound(format!("Unknown device: {device_id}")));
    };

    let current = services::current_status(&latest, Utc::now(), state.config.outage_threshold);

    Ok(Json(StatusResponse {
        device_id,
        state: current.state,
        since: current.since,
        last_seen_at: latest.last_seen_at,
    }))
}
