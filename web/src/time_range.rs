use chrono::{DateTime, Datelike, FixedOffset, NaiveTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::config::WEEK_START;

pub fn start_of_today(now: DateTime<Utc>, offset: FixedOffset) -> DateTime<Utc> {
    let local_midnight = now
        .with_timezone(&offset)
        .date_naive()
        .and_time(NaiveTime::MIN);

    local_midnight
        .and_local_timezone(offset)
        .single()
        .expect("a fixed offset has exactly one local midnight")
        .with_timezone(&Utc)
}

pub fn start_of_week(now: DateTime<Utc>, offset: FixedOffset) -> DateTime<Utc> {
    let days_since_start = now.with_timezone(&offset).weekday().days_since(WEEK_START);

    start_of_today(now, offset) - TimeDelta::days(days_since_start.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PeriodFilter {
    #[serde(rename = "12h")]
    #[default]
    TwelveHour,
    Today,
    ThisWeek,
}

impl PeriodFilter {
    pub fn to_span(&self, now: DateTime<Utc>, offset: FixedOffset) -> Span {
        match self {
            PeriodFilter::TwelveHour => Span {
                start: now - TimeDelta::hours(12),
                end: now,
            },
            PeriodFilter::Today => Span {
                start: start_of_today(now, offset),
                end: now,
            },
            PeriodFilter::ThisWeek => Span {
                start: start_of_week(now, offset),
                end: now,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
pub struct Span {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Span {
    pub fn clip(&self, window: &Span) -> Option<Span> {
        let start = self.start.max(window.start);
        let end = self.end.min(window.end);

        if start < end {
            Some(Span { start, end })
        } else {
            None
        }
    }
    pub fn overlap(&self, window: &Span) -> TimeDelta {
        self.clip(window)
            .map_or(TimeDelta::zero(), |clipped| clipped.duration())
    }
    pub fn duration(&self) -> TimeDelta {
        self.end - self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lagos() -> FixedOffset {
        FixedOffset::east_opt(3600).unwrap()
    }

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn today_starts_at_lagos_midnight() {
        // Wednesday 13:00 in Lagos
        let now = at("2026-10-14T12:00:00Z");
        assert_eq!(start_of_today(now, lagos()), at("2026-10-13T23:00:00Z"));
    }

    #[test]
    fn today_uses_lagos_date_not_utc_date() {
        // Still Saturday in UTC, but already Sunday 00:30 in Lagos
        let now = at("2026-10-17T23:30:00Z");
        assert_eq!(start_of_today(now, lagos()), at("2026-10-17T23:00:00Z"));
    }

    #[test]
    fn today_just_before_lagos_midnight() {
        // Saturday 23:59:59 in Lagos
        let now = at("2026-10-17T22:59:59Z");
        assert_eq!(start_of_today(now, lagos()), at("2026-10-16T23:00:00Z"));
    }

    #[test]
    fn today_exactly_at_lagos_midnight() {
        let now = at("2026-10-17T23:00:00Z");
        assert_eq!(start_of_today(now, lagos()), now);
    }

    #[test]
    fn week_starts_on_sunday() {
        // Wednesday in Lagos, so the week began Sunday 2026-10-11
        let now = at("2026-10-14T12:00:00Z");
        assert_eq!(start_of_week(now, lagos()), at("2026-10-10T23:00:00Z"));
    }

    #[test]
    fn week_on_sunday_is_today() {
        // Sunday 00:30 in Lagos, the first day of a new week
        let now = at("2026-10-17T23:30:00Z");
        assert_eq!(start_of_week(now, lagos()), at("2026-10-17T23:00:00Z"));
    }

    #[test]
    fn week_on_saturday_goes_back_six_days() {
        // Saturday 23:59:59 in Lagos, the last moment of the week
        let now = at("2026-10-17T22:59:59Z");
        assert_eq!(start_of_week(now, lagos()), at("2026-10-10T23:00:00Z"));
    }

    /// A span between two "HH:MM" times on the same UTC day
    fn span(start: &str, end: &str) -> Span {
        Span {
            start: at(&format!("2026-10-14T{start}:00Z")),
            end: at(&format!("2026-10-14T{end}:00Z")),
        }
    }

    /// Every clip/overlap test uses this 10:00–14:00 window
    fn window() -> Span {
        span("10:00", "14:00")
    }

    #[test]
    fn inside_window_is_kept_whole() {
        let period = span("11:00", "12:30");
        assert_eq!(period.clip(&window()), Some(period));
        assert_eq!(period.overlap(&window()), TimeDelta::minutes(90));
    }

    #[test]
    fn starting_before_window_is_cut_at_start() {
        let period = span("08:00", "11:00");
        assert_eq!(period.clip(&window()), Some(span("10:00", "11:00")));
        assert_eq!(period.overlap(&window()), TimeDelta::hours(1));
    }

    #[test]
    fn ending_after_window_is_cut_at_end() {
        let period = span("13:00", "16:00");
        assert_eq!(period.clip(&window()), Some(span("13:00", "14:00")));
        assert_eq!(period.overlap(&window()), TimeDelta::hours(1));
    }

    #[test]
    fn covering_window_becomes_window() {
        let period = span("08:00", "16:00");
        assert_eq!(period.clip(&window()), Some(window()));
        assert_eq!(period.overlap(&window()), TimeDelta::hours(4));
    }

    #[test]
    fn entirely_before_window_is_nothing() {
        let period = span("06:00", "08:00");
        assert_eq!(period.clip(&window()), None);
        assert_eq!(period.overlap(&window()), TimeDelta::zero());
    }

    #[test]
    fn entirely_after_window_is_nothing() {
        let period = span("15:00", "16:00");
        assert_eq!(period.clip(&window()), None);
        assert_eq!(period.overlap(&window()), TimeDelta::zero());
    }

    #[test]
    fn ending_exactly_at_window_start_is_nothing() {
        let period = span("08:00", "10:00");
        assert_eq!(period.clip(&window()), None);
        assert_eq!(period.overlap(&window()), TimeDelta::zero());
    }

    #[test]
    fn starting_exactly_at_window_end_is_nothing() {
        let period = span("14:00", "15:00");
        assert_eq!(period.clip(&window()), None);
        assert_eq!(period.overlap(&window()), TimeDelta::zero());
    }
}
