use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;

use crate::{
    error::AppError,
    models::{BeatRequest, PeriodQuery, PeriodResponse, StatusResponse, SummaryResponse},
    services::{self, BeatAction},
    state::AppState,
    time_range,
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
        BeatAction::Update { id } => services::update_power_period(&mut tx, id, now).await?,
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

pub async fn summary(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<String>,
) -> Result<Json<SummaryResponse>, AppError> {
    let mut conn = state.db.acquire().await?;

    let Some(latest) = services::fetch_latest_period(&mut conn, &device_id).await? else {
        return Err(AppError::NotFound(format!("Unknown device: {device_id}")));
    };

    // One `now` for everything, so the state and both totals agree
    let now = Utc::now();
    let threshold = state.config.outage_threshold;
    let offset = state.config.utc_offset;

    let current = services::current_status(&latest, now, threshold);

    let today_start = time_range::start_of_today(now, offset);
    let week_start = time_range::start_of_week(now, offset);

    let today =
        services::get_total_power(&mut conn, &device_id, today_start, now, now, threshold).await?;
    let week =
        services::get_total_power(&mut conn, &device_id, week_start, now, now, threshold).await?;

    Ok(Json(SummaryResponse {
        device_id,
        state: current.state,
        since: current.since,
        last_seen_at: latest.last_seen_at,
        today_secs: today.num_seconds(),
        week_secs: week.num_seconds(),
    }))
}

pub async fn periods(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<String>,
    Query(period): Query<PeriodQuery>,
) -> Result<Json<PeriodResponse>, AppError> {
    let mut conn = state.db.acquire().await?;

    // One `now` for the window and the spans, so they agree
    let now = Utc::now();
    let window = period.filter.to_span(now, state.config.utc_offset);

    let spans = services::spans_in_window(
        &mut conn,
        &device_id,
        window.start,
        window.end,
        now,
        state.config.outage_threshold,
    )
    .await?;

    // Clip the spans to the requested window, and return them
    let clipped_spans: Vec<time_range::Span> = spans
        .into_iter()
        .filter_map(|span| span.clip(&window))
        .collect();

    Ok(Json(PeriodResponse {
        spans: clipped_spans,
        from: window.start,
        to: window.end,
    }))
}
