#![allow(dead_code)]

use chrono::{DateTime, Datelike, Days, Duration, Local, LocalResult, TimeZone, Utc};

use crate::error::CfdError;
use crate::types::RoundingMode;

pub fn resolve_list_datetime(flag_name: &str, value: &str) -> Result<String, CfdError> {
    resolve_list_datetime_at(flag_name, value, Local::now())
}

pub fn round_timestamp(value: &str, mode: RoundingMode) -> Result<String, CfdError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| CfdError::message(format!("invalid timestamp: {value}")))?;
    Ok(round_datetime(parsed.with_timezone(&Utc), mode).to_rfc3339())
}

fn resolve_list_datetime_at(
    flag_name: &str,
    value: &str,
    now: DateTime<Local>,
) -> Result<String, CfdError> {
    match value {
        "today" => keyword_boundary(flag_name, now, 0),
        "yesterday" => keyword_boundary(flag_name, now, -1),
        _ => chrono::DateTime::parse_from_rfc3339(value)
            .map(|parsed| parsed.to_rfc3339())
            .map_err(|_| CfdError::message(format!("invalid {flag_name}: {value}"))),
    }
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

    let local_dt = match Local.with_ymd_and_hms(
        target_date.year(),
        target_date.month(),
        target_date.day(),
        0,
        0,
        0,
    ) {
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
    fn rejects_invalid_timestamp() {
        let error = resolve_list_datetime_at("start", "not-a-date", Local::now())
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid start"));
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
