use chrono::Duration;

use crate::error::CfdError;

pub fn parse_duration(value: &str) -> Result<Duration, CfdError> {
    if value.trim().is_empty() {
        return Err(CfdError::message("invalid duration: empty"));
    }

    if value.chars().all(|char| char.is_ascii_digit()) {
        let minutes = value
            .parse::<i64>()
            .map_err(|_| CfdError::message(format!("invalid duration: {value}")))?;
        return Ok(Duration::minutes(minutes));
    }

    let mut total_minutes = 0_i64;
    let mut digits = String::new();

    for char in value.chars() {
        if char.is_ascii_digit() {
            digits.push(char);
            continue;
        }

        let amount = digits
            .parse::<i64>()
            .map_err(|_| CfdError::message(format!("invalid duration: {value}")))?;
        digits.clear();

        match char {
            'h' => total_minutes += amount * 60,
            'm' => total_minutes += amount,
            _ => return Err(CfdError::message(format!("invalid duration: {value}"))),
        }
    }

    if !digits.is_empty() {
        return Err(CfdError::message(format!("invalid duration: {value}")));
    }

    Ok(Duration::minutes(total_minutes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_durations() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("2h30m").unwrap(), Duration::minutes(150));
        assert_eq!(parse_duration("90").unwrap(), Duration::minutes(90));
    }

    #[test]
    fn rejects_invalid_durations() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("2hm").is_err());
        assert!(parse_duration("30x").is_err());
    }
}
