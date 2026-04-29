#![allow(dead_code)]

use chrono::{DateTime, Datelike, Days, Duration, Local, LocalResult, TimeZone, Utc, Weekday};

use crate::error::CfdError;
use crate::types::RoundingMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekStart {
    Monday,
    Sunday,
}

pub fn resolve_list_datetime(flag_name: &str, value: &str) -> Result<String, CfdError> {
    resolve_list_datetime_at(flag_name, value, Local::now())
}

pub fn local_today_bounds() -> Result<(String, String), CfdError> {
    local_today_bounds_at(Local::now())
}

pub fn local_week_bounds(week_start: WeekStart) -> Result<(String, String), CfdError> {
    local_week_bounds_at(Local::now(), week_start)
}

pub fn round_timestamp(value: &str, mode: RoundingMode) -> Result<String, CfdError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| CfdError::message(format!("invalid timestamp: {value}")))?;
    Ok(round_datetime(parsed.with_timezone(&Utc), mode).to_rfc3339())
}

pub fn resolve_and_round_timestamp(
    flag_name: &str,
    value: &str,
    mode: RoundingMode,
) -> Result<String, CfdError> {
    let resolved = resolve_timestamp(flag_name, value)?;
    round_timestamp(&resolved, mode)
}

pub fn resolve_and_round_existing_timestamp(
    flag_name: &str,
    value: &str,
    base: Option<&str>,
    mode: RoundingMode,
) -> Result<String, CfdError> {
    let resolved = if is_bare_relative(value) {
        let base = base.ok_or_else(|| {
            CfdError::message(format!(
                "entry update cannot adjust missing {flag_name} time; use --{flag_name} now-5m or --duration <d>"
            ))
        })?;
        resolve_relative_to_base(flag_name, value, base)?
    } else {
        resolve_timestamp(flag_name, value)?
    };

    round_timestamp(&resolved, mode)
}

pub fn resolve_timestamp(flag_name: &str, value: &str) -> Result<String, CfdError> {
    resolve_timestamp_at(flag_name, value, Local::now())
}

fn resolve_timestamp_at(
    flag_name: &str,
    value: &str,
    now: DateTime<Local>,
) -> Result<String, CfdError> {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.to_rfc3339());
    }

    if value == "now" {
        return Ok(now.with_timezone(&Utc).to_rfc3339());
    }

    if let Some(relative) = value.strip_prefix("now") {
        let offset = parse_signed_duration(flag_name, relative, value)?;
        return Ok((now.with_timezone(&Utc) + offset).to_rfc3339());
    }

    if is_bare_relative(value) {
        let offset = parse_signed_duration(flag_name, value, value)?;
        return Ok((now.with_timezone(&Utc) + offset).to_rfc3339());
    }

    Err(CfdError::message(format!("invalid {flag_name}: {value}")))
}

fn resolve_relative_to_base(flag_name: &str, value: &str, base: &str) -> Result<String, CfdError> {
    let base = chrono::DateTime::parse_from_rfc3339(base)
        .map_err(|_| CfdError::message(format!("invalid {flag_name}: {base}")))?;
    let offset = parse_signed_duration(flag_name, value, value)?;
    Ok((base + offset).to_rfc3339())
}

fn resolve_list_datetime_at(
    flag_name: &str,
    value: &str,
    now: DateTime<Local>,
) -> Result<String, CfdError> {
    match value {
        "today" => keyword_boundary(flag_name, now, 0),
        "yesterday" => keyword_boundary(flag_name, now, -1),
        _ => resolve_timestamp_at(flag_name, value, now),
    }
}

fn is_bare_relative(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'+' | b'-'))
}

fn parse_signed_duration(
    flag_name: &str,
    value: &str,
    display_value: &str,
) -> Result<Duration, CfdError> {
    let (sign, body) = value
        .split_at_checked(1)
        .ok_or_else(|| CfdError::message(format!("invalid {flag_name}: {display_value}")))?;
    if sign != "+" && sign != "-" {
        return Err(CfdError::message(format!(
            "invalid {flag_name}: {display_value}"
        )));
    }
    if body.is_empty() || !body.chars().any(|char| matches!(char, 'h' | 'm')) {
        return Err(CfdError::message(format!(
            "invalid {flag_name}: {display_value}"
        )));
    }

    let duration = crate::duration::parse_duration(body)
        .map_err(|_| CfdError::message(format!("invalid {flag_name}: {display_value}")))?;
    if sign == "-" {
        Ok(-duration)
    } else {
        Ok(duration)
    }
}

fn local_today_bounds_at(now: DateTime<Local>) -> Result<(String, String), CfdError> {
    let start_date = now.date_naive();
    let end_date = start_date
        .checked_add_days(Days::new(1))
        .ok_or_else(|| CfdError::message("invalid end: date overflow"))?;

    Ok((
        local_midnight_utc("start", start_date)?,
        local_midnight_utc("end", end_date)?,
    ))
}

fn local_week_bounds_at(
    now: DateTime<Local>,
    week_start: WeekStart,
) -> Result<(String, String), CfdError> {
    let start_weekday = match week_start {
        WeekStart::Monday => Weekday::Mon,
        WeekStart::Sunday => Weekday::Sun,
    };
    let days_since_start = (7 + weekday_number(now.weekday()) - weekday_number(start_weekday)) % 7;
    let start_date = now
        .date_naive()
        .checked_sub_days(Days::new(days_since_start.into()))
        .ok_or_else(|| CfdError::message("invalid start: date overflow"))?;
    let end_date = start_date
        .checked_add_days(Days::new(7))
        .ok_or_else(|| CfdError::message("invalid end: date overflow"))?;

    Ok((
        local_midnight_utc("start", start_date)?,
        local_midnight_utc("end", end_date)?,
    ))
}

fn weekday_number(weekday: Weekday) -> u32 {
    weekday.num_days_from_monday()
}

fn keyword_boundary(
    flag_name: &str,
    now: DateTime<Local>,
    day_offset: i64,
) -> Result<String, CfdError> {
    let base = now.date_naive();
    let shifted = if day_offset >= 0 {
        base.checked_add_days(Days::new(day_offset as u64))
    } else {
        base.checked_sub_days(Days::new((-day_offset) as u64))
    }
    .ok_or_else(|| CfdError::message(format!("invalid {flag_name}: date overflow")))?;

    let target_date = match flag_name {
        "start" => shifted,
        "end" if day_offset == 0 => shifted
            .checked_add_days(Days::new(1))
            .ok_or_else(|| CfdError::message(format!("invalid {flag_name}: date overflow")))?,
        "end" => base,
        _ => {
            return Err(CfdError::message(format!(
                "unsupported datetime flag: {flag_name}"
            )))
        }
    };

    local_midnight_utc(flag_name, target_date)
}

fn local_midnight_utc(flag_name: &str, date: chrono::NaiveDate) -> Result<String, CfdError> {
    let local_dt = match Local.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0) {
        LocalResult::Single(value) => value,
        _ => {
            return Err(CfdError::message(format!(
                "invalid {flag_name}: failed to resolve local midnight"
            )))
        }
    };

    Ok(local_dt.with_timezone(&Utc).to_rfc3339())
}

fn round_datetime(datetime: DateTime<Utc>, mode: RoundingMode) -> DateTime<Utc> {
    let step_minutes = match mode {
        RoundingMode::Off => return datetime,
        RoundingMode::OneMinute => 1_i64,
        RoundingMode::FiveMinutes => 5_i64,
        RoundingMode::TenMinutes => 10_i64,
        RoundingMode::FifteenMinutes => 15_i64,
    };

    let step_seconds = step_minutes * 60;
    let timestamp = datetime.timestamp();
    let remainder = timestamp.rem_euclid(step_seconds);
    let down = datetime - Duration::seconds(remainder);
    if remainder * 2 >= step_seconds {
        down + Duration::seconds(step_seconds)
    } else {
        down
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, LocalResult, TimeZone};

    use super::*;

    fn local_midnight_utc(year: i32, month: u32, day: u32) -> String {
        match Local.with_ymd_and_hms(year, month, day, 0, 0, 0) {
            LocalResult::Single(value) => value.with_timezone(&Utc).to_rfc3339(),
            _ => panic!("failed to resolve local midnight for test"),
        }
    }

    #[test]
    fn resolves_today_for_start() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let result = resolve_list_datetime_at("start", "today", now).unwrap();

        assert_eq!(result, local_midnight_utc(2026, 4, 23));
    }

    #[test]
    fn resolves_yesterday_for_end() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let result = resolve_list_datetime_at("end", "yesterday", now).unwrap();

        assert_eq!(
            result,
            local_midnight_utc(now.year(), now.month(), now.day())
        );
    }

    #[test]
    fn resolves_local_today_bounds() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let (start, end) = local_today_bounds_at(now).unwrap();

        assert_eq!(start, local_midnight_utc(2026, 4, 23));
        assert_eq!(end, local_midnight_utc(2026, 4, 24));
    }

    #[test]
    fn resolves_monday_week_bounds() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let (start, end) = local_week_bounds_at(now, WeekStart::Monday).unwrap();

        assert_eq!(start, local_midnight_utc(2026, 4, 20));
        assert_eq!(end, local_midnight_utc(2026, 4, 27));
    }

    #[test]
    fn resolves_sunday_week_bounds() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let (start, end) = local_week_bounds_at(now, WeekStart::Sunday).unwrap();

        assert_eq!(start, local_midnight_utc(2026, 4, 19));
        assert_eq!(end, local_midnight_utc(2026, 4, 26));
    }

    #[test]
    fn rejects_invalid_timestamp() {
        let error = resolve_list_datetime_at("start", "not-a-date", Local::now())
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid start"));
    }

    #[test]
    fn resolves_relative_timestamps_from_now() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        assert_eq!(
            resolve_timestamp_at("start", "now", now).unwrap(),
            now.with_timezone(&Utc).to_rfc3339()
        );
        assert_eq!(
            resolve_timestamp_at("start", "-3m", now).unwrap(),
            (now.with_timezone(&Utc) - Duration::minutes(3)).to_rfc3339()
        );
        assert_eq!(
            resolve_timestamp_at("end", "+30m", now).unwrap(),
            (now.with_timezone(&Utc) + Duration::minutes(30)).to_rfc3339()
        );
        assert_eq!(
            resolve_timestamp_at("start", "now-2h", now).unwrap(),
            (now.with_timezone(&Utc) - Duration::hours(2)).to_rfc3339()
        );
        assert_eq!(
            resolve_timestamp_at("end", "now+1h30m", now).unwrap(),
            (now.with_timezone(&Utc) + Duration::minutes(90)).to_rfc3339()
        );
    }

    #[test]
    fn list_datetime_accepts_relative_timestamps() {
        let now = match Local.with_ymd_and_hms(2026, 4, 23, 15, 30, 0) {
            LocalResult::Single(value) => value,
            _ => panic!("failed to resolve local test time"),
        };

        let result = resolve_list_datetime_at("start", "-2h", now).unwrap();

        assert_eq!(
            result,
            (now.with_timezone(&Utc) - Duration::hours(2)).to_rfc3339()
        );
    }

    #[test]
    fn resolves_bare_relative_timestamps_against_existing_base() {
        assert_eq!(
            resolve_relative_to_base("end", "-5m", "2026-04-23T10:00:00Z").unwrap(),
            "2026-04-23T09:55:00+00:00"
        );
        assert_eq!(
            resolve_relative_to_base("start", "+10m", "2026-04-23T09:00:00Z").unwrap(),
            "2026-04-23T09:10:00+00:00"
        );
    }

    #[test]
    fn rejects_invalid_relative_timestamps() {
        for value in ["15m", "nowish", "now--15m", "-", "-2d", "-15"] {
            let error = resolve_timestamp_at("start", value, Local::now())
                .unwrap_err()
                .to_string();

            assert!(
                error.contains("invalid start"),
                "unexpected error for {value}: {error}"
            );
        }
    }

    #[test]
    fn rounding_uses_half_up_semantics() {
        assert_eq!(
            round_timestamp("2026-04-23T14:21:00Z", RoundingMode::FifteenMinutes).unwrap(),
            "2026-04-23T14:15:00+00:00"
        );
        assert_eq!(
            round_timestamp("2026-04-23T14:25:00Z", RoundingMode::FifteenMinutes).unwrap(),
            "2026-04-23T14:30:00+00:00"
        );
    }

    #[test]
    fn rounding_crosses_hour_and_day_boundaries() {
        assert_eq!(
            round_timestamp("2026-04-23T23:58:00Z", RoundingMode::FiveMinutes).unwrap(),
            "2026-04-24T00:00:00+00:00"
        );
        assert_eq!(
            round_timestamp("2026-04-23T10:59:31Z", RoundingMode::OneMinute).unwrap(),
            "2026-04-23T11:00:00+00:00"
        );
    }

    #[test]
    fn rounding_allows_future_results() {
        let rounded = round_timestamp("2099-12-31T23:59:31Z", RoundingMode::OneMinute).unwrap();
        assert_eq!(rounded, "2100-01-01T00:00:00+00:00");
    }
}
