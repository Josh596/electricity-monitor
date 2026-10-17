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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PowerPeriod {
    pub id: i64,
    pub device_id: String,
    pub boot_id: u32,
    pub reset_reason: Option<ResetReason>,
    pub started_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
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
