-- Add migration script here
CREATE TABLE power_periods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    boot_id INTEGER NOT NULL,
    reset_reason TEXT NULL,
    started_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);

CREATE INDEX idx_power_periods_device_started ON power_periods (device_id, started_at);