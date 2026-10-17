use chrono::{DateTime, TimeDelta, Utc};
use sqlx::SqliteConnection;

use crate::{
    models::{PowerState, ResetReason},
    time_range::Span,
};

#[derive(Debug, Clone)]
pub struct LatestPeriod {
    pub id: i64,
    pub boot_id: u32,
    pub started_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub struct CurrentStatus {
    pub state: PowerState,
    /// When the current state began
    pub since: DateTime<Utc>,
}

pub fn current_status(
    latest: &LatestPeriod,
    now: DateTime<Utc>,
    outage_threshold: TimeDelta,
) -> CurrentStatus {
    if now - latest.last_seen_at > outage_threshold {
        // Power went off somewhere after the last beat
        CurrentStatus {
            state: PowerState::Off,
            since: latest.last_seen_at,
        }
    } else {
        CurrentStatus {
            state: PowerState::On,
            since: latest.started_at,
        }
    }
}

/// Turns power periods (ordered oldest first) into spans. Every period ends at its last beat,
/// except the latest one while the power is still on, which runs until now.
pub fn periods_to_spans(
    periods: &[LatestPeriod],
    state: PowerState,
    now: DateTime<Utc>,
) -> Vec<Span> {
    let mut spans: Vec<Span> = periods
        .iter()
        .map(|p| Span {
            start: p.started_at,
            end: p.last_seen_at,
        })
        .collect();

    if state == PowerState::On
        && let Some(latest) = spans.last_mut()
    {
        latest.end = now;
    }

    spans
}

#[derive(Debug, PartialEq)]
pub enum BeatAction {
    /// New boot (or first beat ever): start a new power period
    Insert { started_at: DateTime<Utc> },
    /// Same boot: extend the existing power period
    Update { id: i64 },
}

pub fn decide_beat_action(
    latest: Option<&LatestPeriod>,
    boot_id: u32,
    uptime_ms: u32,
    now: DateTime<Utc>,
) -> BeatAction {
    match latest {
        Some(last) if last.boot_id == boot_id => BeatAction::Update { id: last.id },
        // The device booted uptime_ms ago, which is when the power came back
        _ => BeatAction::Insert {
            started_at: now - TimeDelta::milliseconds(uptime_ms.into()),
        },
    }
}

pub async fn fetch_latest_period(
    conn: &mut SqliteConnection,
    device_id: &str,
) -> Result<Option<LatestPeriod>, sqlx::Error> {
    sqlx::query_as!(
        LatestPeriod,
        r#"SELECT
            id,
            boot_id as "boot_id: u32",
            started_at as "started_at: DateTime<Utc>",
            last_seen_at as "last_seen_at: DateTime<Utc>"
        FROM power_periods
        WHERE device_id = ?
        ORDER BY started_at DESC
        LIMIT 1"#,
        device_id,
    )
    .fetch_optional(conn)
    .await
}

pub async fn insert_power_period(
    conn: &mut SqliteConnection,
    device_id: &str,
    boot_id: u32,
    reset_reason: Option<ResetReason>,
    started_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO power_periods (device_id, boot_id, reset_reason, started_at, last_seen_at) VALUES (?, ?, ?, ?, ?)",
        device_id,
        boot_id,
        reset_reason as _,
        started_at,
        now,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_power_period(
    conn: &mut SqliteConnection,
    id: i64,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE power_periods SET last_seen_at = ? WHERE id = ?",
        now,
        id,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn fetch_periods_between(
    conn: &mut SqliteConnection,
    device_id: &str,
    a: DateTime<Utc>,
    b: DateTime<Utc>,
) -> Result<Vec<LatestPeriod>, sqlx::Error> {
    sqlx::query_as!(
        LatestPeriod,
        r#"
        SELECT 
            id, 
            boot_id as "boot_id: u32", 
            started_at as "started_at: DateTime<Utc>", 
            last_seen_at as "last_seen_at: DateTime<Utc>" 
        FROM power_periods 
        WHERE device_id = ? AND started_at < ? AND last_seen_at > ? ORDER BY started_at ASC"#,
        device_id,
        b,
        a
    )
    .fetch_all(conn)
    .await
}

pub async fn get_total_power(
    conn: &mut SqliteConnection,
    device_id: &str,
    a: DateTime<Utc>,
    b: DateTime<Utc>,
    now: DateTime<Utc>,
    outage_threshold: TimeDelta,
) -> Result<TimeDelta, sqlx::Error> {
    let window = Span { start: a, end: b };

    let spans = spans_in_window(conn, device_id, a, b, now, outage_threshold).await?;

    let total = spans.iter().map(|span| span.overlap(&window)).sum();

    Ok(total)
}

pub async fn spans_in_window(
    conn: &mut SqliteConnection,
    device_id: &str,
    a: DateTime<Utc>,
    b: DateTime<Utc>,
    now: DateTime<Utc>,
    outage_threshold: TimeDelta,
) -> Result<Vec<Span>, sqlx::Error> {
    let periods = fetch_periods_between(conn, device_id, a, b).await?;

    let Some(latest) = periods.last() else {
        return Ok(vec![]);
    };

    let state = current_status(latest, now, outage_threshold).state;

    let spans = periods_to_spans(&periods, state, now);

    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-10-15T12:00:00Z".parse().unwrap()
    }

    /// A period with boot_id 7 that started an hour ago and was last seen `seen_ago` ago
    fn period(seen_ago: TimeDelta) -> LatestPeriod {
        LatestPeriod {
            id: 3,
            boot_id: 7,
            started_at: now() - TimeDelta::hours(1),
            last_seen_at: now() - seen_ago,
        }
    }

    /// A reboot: an older period that ended 2 minutes before the latest one (from `period`) began
    fn two_periods() -> Vec<LatestPeriod> {
        let older = LatestPeriod {
            id: 2,
            boot_id: 6,
            started_at: now() - TimeDelta::hours(5),
            last_seen_at: now() - TimeDelta::hours(1) - TimeDelta::minutes(2),
        };
        vec![older, period(TimeDelta::seconds(20))]
    }

    #[test]
    fn spans_end_at_last_beat_when_off() {
        let periods = two_periods();
        let spans = periods_to_spans(&periods, PowerState::Off, now());
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].end, periods[0].last_seen_at);
        assert_eq!(spans[1].end, periods[1].last_seen_at);
    }

    #[test]
    fn latest_span_runs_until_now_when_on() {
        let periods = two_periods();
        let spans = periods_to_spans(&periods, PowerState::On, now());
        // The older period is untouched, only the latest one is extended
        assert_eq!(spans[0].end, periods[0].last_seen_at);
        assert_eq!(spans[1].end, now());
    }

    #[test]
    fn no_periods_gives_no_spans() {
        assert!(periods_to_spans(&[], PowerState::On, now()).is_empty());
    }

    #[test]
    fn first_beat_inserts() {
        let action = decide_beat_action(None, 7, 0, now());
        assert_eq!(action, BeatAction::Insert { started_at: now() });
    }

    #[test]
    fn same_boot_updates() {
        let latest = period(TimeDelta::seconds(30));
        let action = decide_beat_action(Some(&latest), 7, 60_000, now());
        assert_eq!(action, BeatAction::Update { id: 3 });
    }

    #[test]
    fn new_boot_inserts_backdated_by_uptime() {
        let uptime_ms = 90_000; // 90 seconds
        let latest = period(TimeDelta::seconds(30));
        let action = decide_beat_action(Some(&latest), 8, uptime_ms, now());
        assert_eq!(
            action,
            BeatAction::Insert {
                started_at: now() - TimeDelta::seconds(uptime_ms as i64 / 1000)
            }
        );
    }

    #[test]
    fn recent_beat_is_on_since_period_start() {
        let latest = period(TimeDelta::seconds(89));
        let status = current_status(&latest, now(), TimeDelta::seconds(90));
        assert_eq!(
            status,
            CurrentStatus {
                state: PowerState::On,
                since: latest.started_at
            }
        );
    }

    #[test]
    fn old_beat_is_off_since_last_seen() {
        let latest = period(TimeDelta::seconds(91));
        let status = current_status(&latest, now(), TimeDelta::seconds(90));
        assert_eq!(
            status,
            CurrentStatus {
                state: PowerState::Off,
                since: latest.last_seen_at
            }
        );
    }
}
