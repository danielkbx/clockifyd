use std::collections::BTreeSet;

use chrono::Utc;
use serde::Serialize;

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::config;
use crate::datetime;
use crate::duration;
use crate::error::CfdError;
use crate::format::{format_json, format_text_fields, OutputFormat, OutputOptions, TextField};
use crate::input;
use crate::types::{EntryFilters, OverlapWarning, StoredConfig, TimeEntry};

const USAGE: &str =
    "usage: cfd split <entry <id>|timer> --at <time> [--gap <duration>] [--no-rounding] [-y]";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SplitResult {
    updated: TimeEntry,
    created: TimeEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SplitTimes {
    split_end: String,
    new_start: String,
}

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    validate_common_flags(args)?;
    match args.action.as_deref() {
        Some("entry") => split_entry(client, args, workspace_id, config_state),
        Some("timer") => split_timer(client, args, workspace_id, config_state),
        _ => Err(CfdError::message(USAGE)),
    }
}

fn split_entry<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    let entry_id = match args.positional.as_slice() {
        [entry_id] => entry_id,
        _ => {
            return Err(CfdError::message(
                "usage: cfd split entry <id> --at <time> [--gap <duration>]",
            ))
        }
    };

    let user = client.get_current_user()?;
    let existing = client.get_time_entry(workspace_id, entry_id)?;
    let original_end = existing.time_interval.end.clone().ok_or_else(|| {
        CfdError::message(
            "entry split requires a finished entry; use cfd split timer for the running timer",
        )
    })?;
    let times = split_times(args, config_state)?;

    let original_start_dt = parse_rfc3339("entry start", &existing.time_interval.start)?;
    let original_end_dt = parse_rfc3339("entry end", &original_end)?;
    let split_end_dt = parse_rfc3339("split time", &times.split_end)?;
    let new_start_dt = parse_rfc3339("new entry start", &times.new_start)?;

    if split_end_dt <= original_start_dt {
        return Err(CfdError::message(
            "split time must be after entry start; if this came from rounding, retry with --no-rounding",
        ));
    }
    if split_end_dt >= original_end_dt {
        return Err(CfdError::message(
            "split time must be before entry end; if this came from rounding, retry with --no-rounding",
        ));
    }
    if new_start_dt >= original_end_dt {
        return Err(CfdError::message(
            "new entry start must be before original entry end; reduce --gap or retry with --no-rounding",
        ));
    }

    let update_payload = entry_payload(
        &existing,
        &existing.time_interval.start,
        Some(&times.split_end),
    )?;
    let create_payload = entry_payload(&existing, &times.new_start, Some(&original_end))?;

    let update_warning = find_overlaps(
        client,
        workspace_id,
        &user.id,
        &existing.time_interval.start,
        Some(&times.split_end),
        Some(entry_id),
    )?;
    let create_warning = find_overlaps(
        client,
        workspace_id,
        &user.id,
        &times.new_start,
        Some(&original_end),
        Some(entry_id),
    )?;
    maybe_confirm_overlap(
        &combine_warnings([update_warning, create_warning]),
        args.yes,
    )?;

    let updated = client.update_time_entry(workspace_id, entry_id, &update_payload)?;
    let created = client.create_time_entry(workspace_id, &create_payload)?;
    print_result(
        client,
        workspace_id,
        &SplitResult { updated, created },
        &args.output,
    )
}

fn split_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    if !args.positional.is_empty() {
        return Err(CfdError::message(
            "usage: cfd split timer --at <time> [--gap <duration>]",
        ));
    }

    let user = client.get_current_user()?;
    let current = find_current_timer(client, workspace_id, &user.id)?;
    let project_id = current
        .project_id
        .clone()
        .ok_or_else(|| CfdError::message("current timer has no project to split"))?;
    let times = split_times(args, config_state)?;

    let timer_start_dt = parse_rfc3339("timer start", &current.time_interval.start)?;
    let split_end_dt = parse_rfc3339("split time", &times.split_end)?;
    if split_end_dt <= timer_start_dt {
        return Err(CfdError::message(
            "split time must be after timer start; if this came from rounding, retry with --no-rounding",
        ));
    }
    if split_end_dt.with_timezone(&Utc) > Utc::now() {
        return Err(CfdError::message(
            "split time must not be in the future; if this came from rounding, retry with --no-rounding",
        ));
    }

    let stopped_warning = find_overlaps(
        client,
        workspace_id,
        &user.id,
        &current.time_interval.start,
        Some(&times.split_end),
        Some(&current.id),
    )?;
    let started_warning = find_overlaps(
        client,
        workspace_id,
        &user.id,
        &times.new_start,
        None,
        Some(&current.id),
    )?;
    maybe_confirm_overlap(
        &combine_warnings([stopped_warning, started_warning]),
        args.yes,
    )?;

    let updated = client.stop_timer(workspace_id, &user.id, &times.split_end)?;
    let created = client.create_time_entry(
        workspace_id,
        &timer_payload(&current, &project_id, &times.new_start),
    )?;

    if config_state
        .active_switch
        .as_ref()
        .is_some_and(|active_switch| {
            active_switch.workspace_id == workspace_id
                && active_switch.user_id == user.id
                && active_switch.switched_entry_id == current.id
        })
    {
        let mut next_config = config_state.clone();
        if let Some(active_switch) = &mut next_config.active_switch {
            active_switch.switched_entry_id = created.id.clone();
            active_switch.switched_start = times.new_start.clone();
        }
        config::save_config(&next_config)?;
    }

    print_result(
        client,
        workspace_id,
        &SplitResult { updated, created },
        &args.output,
    )
}

fn validate_common_flags(args: &ParsedArgs) -> Result<(), CfdError> {
    if args
        .flags
        .get("at")
        .is_none_or(|value| value == "true" || value.trim().is_empty())
    {
        return Err(CfdError::message(USAGE));
    }
    if args
        .flags
        .get("gap")
        .is_some_and(|value| value == "true" || value.trim().is_empty())
    {
        return Err(CfdError::message("usage: cfd split ... --gap <duration>"));
    }
    for flag in [
        "columns",
        "sort",
        "week-start",
        "project",
        "task",
        "tag",
        "description",
        "start",
        "end",
        "duration",
        "text",
        "name",
        "scope",
    ] {
        if args.flags.contains_key(flag) {
            return Err(CfdError::message(format!(
                "cfd split does not accept --{flag}; {USAGE}"
            )));
        }
    }
    Ok(())
}

fn split_times(args: &ParsedArgs, config_state: &StoredConfig) -> Result<SplitTimes, CfdError> {
    let at = args
        .flags
        .get("at")
        .map(String::as_str)
        .ok_or_else(|| CfdError::message(USAGE))?;
    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let split_end = datetime::resolve_and_round_timestamp("at", at, rounding)?;
    let split_end_dt = parse_rfc3339("split time", &split_end)?;
    let gap = args
        .flags
        .get("gap")
        .map(|value| duration::parse_duration(value))
        .transpose()?
        .unwrap_or_else(chrono::Duration::zero);
    let new_start_unrounded = split_end_dt + gap;
    let new_start = datetime::round_timestamp(&new_start_unrounded.to_rfc3339(), rounding)?;
    Ok(SplitTimes {
        split_end,
        new_start,
    })
}

fn parse_rfc3339(
    label: &str,
    value: &str,
) -> Result<chrono::DateTime<chrono::FixedOffset>, CfdError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| CfdError::message(format!("invalid {label}: {value}")))
}

fn entry_payload(
    entry: &TimeEntry,
    start: &str,
    end: Option<&str>,
) -> Result<serde_json::Value, CfdError> {
    let mut payload = serde_json::json!({
        "start": start,
    });
    if let Some(end) = end {
        payload["end"] = serde_json::Value::String(end.to_owned());
    }
    apply_entry_fields(entry, &mut payload)?;
    Ok(payload)
}

fn timer_payload(entry: &TimeEntry, project_id: &str, start: &str) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "start": start,
        "projectId": project_id,
    });
    if !entry.description.is_empty() {
        payload["description"] = serde_json::Value::String(entry.description.clone());
    }
    if let Some(task_id) = &entry.task_id {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if !entry.tag_ids.is_empty() {
        payload["tagIds"] = entry
            .tag_ids
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect();
    }
    payload
}

fn apply_entry_fields(entry: &TimeEntry, payload: &mut serde_json::Value) -> Result<(), CfdError> {
    payload["description"] = serde_json::Value::String(entry.description.clone());
    if let Some(project_id) = &entry.project_id {
        payload["projectId"] = serde_json::Value::String(project_id.clone());
    }
    if let Some(task_id) = &entry.task_id {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if !entry.tag_ids.is_empty() {
        payload["tagIds"] = entry
            .tag_ids
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect();
    }
    Ok(())
}

fn find_current_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
) -> Result<TimeEntry, CfdError> {
    client
        .get_current_timers(workspace_id)?
        .into_iter()
        .find(|entry| entry.user_id.as_deref() == Some(user_id))
        .ok_or_else(|| CfdError::message("no running timer"))
}

fn find_overlaps<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
    start: &str,
    end: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Option<OverlapWarning>, CfdError> {
    let entries = client.list_time_entries(workspace_id, user_id, &EntryFilters::default())?;
    let overlapping_ids = find_overlapping_ids(&entries, start, end, exclude_id)?;
    if overlapping_ids.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OverlapWarning { overlapping_ids }))
    }
}

fn find_overlapping_ids(
    entries: &[TimeEntry],
    start: &str,
    end: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Vec<String>, CfdError> {
    let start_dt = parse_rfc3339("start", start)?;
    let end_dt = end.map(|value| parse_rfc3339("end", value)).transpose()?;
    let mut overlapping_ids = Vec::new();

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
            overlapping_ids.push(entry.id.clone());
        }
    }

    Ok(overlapping_ids)
}

fn combine_warnings<const N: usize>(
    warnings: [Option<OverlapWarning>; N],
) -> Option<OverlapWarning> {
    let overlapping_ids = warnings
        .into_iter()
        .flatten()
        .flat_map(|warning| warning.overlapping_ids)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    (!overlapping_ids.is_empty()).then_some(OverlapWarning { overlapping_ids })
}

fn maybe_confirm_overlap(warning: &Option<OverlapWarning>, yes: bool) -> Result<(), CfdError> {
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

fn print_result<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    result: &SplitResult,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(result)?),
        OutputFormat::Text => {
            let updated_project_name = project_name(client, workspace_id, &result.updated)?;
            let created_project_name = project_name(client, workspace_id, &result.created)?;
            println!(
                "updated:\n{}\n\ncreated:\n{}",
                format_entry_text(&result.updated, updated_project_name.as_deref(), output)?,
                format_entry_text(&result.created, created_project_name.as_deref(), output)?
            );
        }
    }
    Ok(())
}

fn project_name<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    entry: &TimeEntry,
) -> Result<Option<String>, CfdError> {
    entry
        .project_id
        .as_deref()
        .map(|project_id| client.get_project(workspace_id, project_id))
        .transpose()
        .map(|project| project.map(|project| project.name))
}

fn format_entry_text(
    entry: &TimeEntry,
    project_name: Option<&str>,
    output: &OutputOptions,
) -> Result<String, CfdError> {
    let end = entry.time_interval.end.as_deref().unwrap_or("");
    let duration = entry_duration(entry)?;
    let tags = entry.tag_ids.join(", ");
    Ok(format_text_fields(
        &[
            TextField {
                label: "id",
                value: &entry.id,
                is_meta: true,
            },
            TextField {
                label: "start",
                value: &entry.time_interval.start,
                is_meta: false,
            },
            TextField {
                label: "end",
                value: end,
                is_meta: false,
            },
            TextField {
                label: "duration",
                value: &duration,
                is_meta: false,
            },
            TextField {
                label: "description",
                value: &entry.description,
                is_meta: false,
            },
            TextField {
                label: "projectId",
                value: entry.project_id.as_deref().unwrap_or(""),
                is_meta: false,
            },
            TextField {
                label: "projectName",
                value: project_name.unwrap_or(""),
                is_meta: false,
            },
            TextField {
                label: "task",
                value: entry.task_id.as_deref().unwrap_or(""),
                is_meta: false,
            },
            TextField {
                label: "tags",
                value: &tags,
                is_meta: false,
            },
        ],
        output,
    ))
}

fn entry_duration(entry: &TimeEntry) -> Result<String, CfdError> {
    if let Some(duration) = &entry.time_interval.duration {
        return Ok(duration.clone());
    }
    let Some(end) = entry.time_interval.end.as_deref() else {
        return Ok(String::new());
    };
    let start = parse_rfc3339("entry start", &entry.time_interval.start)?;
    let end = parse_rfc3339("entry end", end)?;
    Ok(crate::commands::timer::format_elapsed(
        end.with_timezone(&Utc)
            .signed_duration_since(start.with_timezone(&Utc)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RoundingMode, TimeInterval};

    fn args(at: &str, gap: Option<&str>, no_rounding: bool) -> ParsedArgs {
        let mut flags = std::collections::HashMap::from([("at".into(), at.into())]);
        if let Some(gap) = gap {
            flags.insert("gap".into(), gap.into());
        }
        ParsedArgs {
            resource: Some("split".into()),
            action: Some("entry".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags,
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding,
        }
    }

    #[test]
    fn split_times_add_gap_to_rounded_end_then_round_new_start() {
        let parsed = args("2026-04-23T10:07:00Z", Some("5m"), false);
        let config = StoredConfig {
            rounding: Some(RoundingMode::FifteenMinutes),
            ..StoredConfig::default()
        };

        let times = split_times(&parsed, &config).unwrap();

        assert_eq!(times.split_end, "2026-04-23T10:00:00+00:00");
        assert_eq!(times.new_start, "2026-04-23T10:00:00+00:00");
    }

    #[test]
    fn split_times_no_rounding_disables_both_rounding_steps() {
        let parsed = args("2026-04-23T10:07:00Z", Some("5m"), true);
        let config = StoredConfig {
            rounding: Some(RoundingMode::FifteenMinutes),
            ..StoredConfig::default()
        };

        let times = split_times(&parsed, &config).unwrap();

        assert_eq!(times.split_end, "2026-04-23T10:07:00+00:00");
        assert_eq!(times.new_start, "2026-04-23T10:12:00+00:00");
    }

    #[test]
    fn combine_warnings_deduplicates_ids() {
        let warning = combine_warnings([
            Some(OverlapWarning {
                overlapping_ids: vec!["b".into(), "a".into()],
            }),
            Some(OverlapWarning {
                overlapping_ids: vec!["a".into(), "c".into()],
            }),
        ])
        .unwrap();

        assert_eq!(warning.overlapping_ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn find_overlapping_ids_excludes_target_entry() {
        let entries = vec![
            TimeEntry {
                id: "self".into(),
                workspace_id: "w1".into(),
                user_id: Some("u1".into()),
                project_id: None,
                task_id: None,
                tag_ids: vec![],
                description: String::new(),
                time_interval: TimeInterval {
                    start: "2026-04-23T09:00:00Z".into(),
                    end: Some("2026-04-23T11:00:00Z".into()),
                    duration: None,
                },
            },
            TimeEntry {
                id: "other".into(),
                workspace_id: "w1".into(),
                user_id: Some("u1".into()),
                project_id: None,
                task_id: None,
                tag_ids: vec![],
                description: String::new(),
                time_interval: TimeInterval {
                    start: "2026-04-23T10:30:00Z".into(),
                    end: Some("2026-04-23T11:30:00Z".into()),
                    duration: None,
                },
            },
        ];

        let ids = find_overlapping_ids(
            &entries,
            "2026-04-23T10:00:00Z",
            Some("2026-04-23T11:00:00Z"),
            Some("self"),
        )
        .unwrap();

        assert_eq!(ids, vec!["other"]);
    }
}
