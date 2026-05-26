use std::collections::BTreeSet;

use crate::client::{ClockifyClient, HttpTransport};
use crate::datetime::parse_rfc3339;
use crate::error::CfdError;
use crate::input;
use crate::types::{EntryFilters, OverlapWarning, TimeEntry};

/// Loads the current user's time entries and returns an [`OverlapWarning`]
/// for any that intersect `[start, end)`. `exclude_id` skips the entry being
/// modified so it does not collide with itself.
pub fn detect<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
    start: &str,
    end: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Option<OverlapWarning>, CfdError> {
    let filters = end
        .map(|end| EntryFilters {
            start: Some(start.to_owned()),
            end: Some(end.to_owned()),
            ..EntryFilters::default()
        })
        .unwrap_or_default();
    let entries = client.list_all_time_entries(workspace_id, user_id, &filters)?;
    let overlapping_ids = detect_in(&entries, start, end, exclude_id)?;
    Ok((!overlapping_ids.is_empty()).then_some(OverlapWarning { overlapping_ids }))
}

/// Pure overlap calculation over a fixed entry slice — useful for tests and
/// when callers already have the entries in hand.
pub fn detect_in(
    entries: &[TimeEntry],
    start: &str,
    end: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Vec<String>, CfdError> {
    let start_dt = parse_rfc3339("start", start)?;
    let end_dt = end.map(|value| parse_rfc3339("end", value)).transpose()?;

    let mut overlapping = Vec::new();
    for entry in entries {
        if exclude_id == Some(entry.id.as_str()) {
            continue;
        }
        let existing_start = parse_rfc3339("existing start", &entry.time_interval.start)?;
        let existing_end = entry
            .time_interval
            .end
            .as_deref()
            .map(|value| parse_rfc3339("existing end", value))
            .transpose()?;

        let overlaps = match (end_dt, existing_end) {
            (Some(new_end), Some(existing_end)) => {
                existing_start < new_end && start_dt < existing_end
            }
            (Some(new_end), None) => existing_start < new_end,
            (None, Some(existing_end)) => start_dt < existing_end,
            (None, None) => true,
        };

        if overlaps {
            overlapping.push(entry.id.clone());
        }
    }

    Ok(overlapping)
}

/// Merges multiple optional warnings (e.g. from a split that mutates two entries)
/// into one, deduplicating and sorting the resulting ids.
pub fn combine<const N: usize>(warnings: [Option<OverlapWarning>; N]) -> Option<OverlapWarning> {
    let overlapping_ids: Vec<String> = warnings
        .into_iter()
        .flatten()
        .flat_map(|warning| warning.overlapping_ids)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    (!overlapping_ids.is_empty()).then_some(OverlapWarning { overlapping_ids })
}

/// Prints the overlap warning to stderr and asks for confirmation unless `yes`
/// is set. `-y` skips the prompt but never the detection.
pub fn confirm(warning: &Option<OverlapWarning>, yes: bool) -> Result<(), CfdError> {
    if let Some(warning) = warning {
        eprintln!(
            "warning: overlaps existing entries: {}",
            warning.overlapping_ids.join(", ")
        );
        if !yes && !input::confirm("Continue despite overlap?")? {
            return Err(CfdError::message("aborted due to overlap"));
        }
    }
    Ok(())
}
