use crate::time_range::{PeriodFilter, Span};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum ResetReason {
    PowerOn,
    Brownout,
    Panic,
    Watchdog,
    Software,
    External,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PeriodQuery {
    #[serde(default)]
    pub filter: PeriodFilter,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodResponse {
    pub spans: Vec<Span>,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BeatRequest {
    pub device_id: String,
    pub boot_id: u32,
    pub reset_reason: Option<ResetReason>,
    pub uptime_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub device_id: String,
    pub state: PowerState,
    pub since: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryResponse {
    pub device_id: String,
    pub state: PowerState,
    pub since: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    /// Seconds of power since Lagos midnight
    pub today_secs: i64,
    /// Seconds of power since the start of the week (Sunday, Lagos midnight)
    pub week_secs: i64,
}
