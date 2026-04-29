use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::config;
use crate::datetime;
use crate::duration;
use crate::error::CfdError;
use crate::format::{
    format_entry_text_items, format_json, format_resource_id, format_text_blocks,
    format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::input;
use crate::types::{EntryFilters, EntryTextItem, StoredConfig, TimeEntry};
use chrono::{DateTime, FixedOffset};
use std::collections::BTreeMap;

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_entry_view_flags(args)?;
            list_entries(client, args, workspace_id)
        }
        Some("get") => {
            validate_entry_view_flags(args)?;
            get_entry(client, args, workspace_id)
        }
        Some("add") => add_entry(client, args, workspace_id, config_state),
        Some("update") => update_entry(client, args, workspace_id, config_state),
        Some("delete") => delete_entry(client, args, workspace_id),
        Some("text") if args.subaction.as_deref() == Some("list") => {
            list_entry_texts(client, args, workspace_id, config_state)
        }
        Some("text") => Err(CfdError::message(
            "usage: cfd entry text list [--project <id>]",
        )),
        _ => Err(CfdError::message(
            "usage: cfd entry <list|get|add|update|delete>",
        )),
    }
}

fn list_entries<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let filters = filters_from_args(args)?;
    let columns = parse_entry_columns(args.flags.get("columns").map(String::as_str))?;
    let sort = parse_entry_sort(
        args.flags.get("sort").map(String::as_str),
        "usage: cfd entry list ... --sort <asc|desc>",
    )?;
    let user = client.get_current_user()?;
    let entries = sort_entries(
        client.list_time_entries(workspace_id, &user.id, &filters)?,
        sort,
    )?;

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&entries)?),
        OutputFormat::Text => {
            let project_names = load_project_names_for_entries(client, workspace_id, &entries)?;
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &entries
                            .iter()
                            .map(|entry| {
                                format_entry_text(
                                    entry,
                                    project_names.get(entry.project_id.as_deref().unwrap_or("")),
                                    &args.output,
                                    &columns,
                                )
                            })
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_entry_table(&entries, &project_names, &columns));
            }
        }
    }

    Ok(())
}

fn get_entry<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let entry_id = args
        .positional
        .first()
        .ok_or_else(|| CfdError::message("usage: cfd entry get <id>"))?;
    let columns = parse_entry_columns(args.flags.get("columns").map(String::as_str))?;
    let entry = client.get_time_entry(workspace_id, entry_id)?;

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&entry)?),
        OutputFormat::Text => {
            let project_name = entry
                .project_id
                .as_deref()
                .map(|project_id| client.get_project(workspace_id, project_id))
                .transpose()?
                .map(|project| project.name);
            if columns.is_empty() {
                println!(
                    "{}",
                    format_entry_text(&entry, project_name.as_ref(), &args.output, &columns)
                );
            } else {
                let project_names = project_name
                    .as_ref()
                    .zip(entry.project_id.as_ref())
                    .map(|(name, id)| BTreeMap::from([(id.clone(), name.clone())]))
                    .unwrap_or_default();
                println!("{}", format_entry_table(&[entry], &project_names, &columns));
            }
        }
    }

    Ok(())
}

fn add_entry<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;
    let payload = build_time_entry_payload(args, config_state)?;
    let warning = find_overlaps_for_payload(
        client,
        workspace_id,
        &user.id,
        None,
        payload["start"].as_str().unwrap(),
        payload["end"].as_str().unwrap(),
    )?;
    maybe_confirm_overlap(&warning, args.yes)?;
    let entry = client.create_time_entry(workspace_id, &payload)?;
    println!("{}", format_resource_id(&entry.id));
    Ok(())
}

fn update_entry<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    let entry_id = args.positional.first().ok_or_else(|| {
        CfdError::message(
            "usage: cfd entry update <id> [--start <time>] [--end <time> | --duration <d>] [fields...]",
        )
    })?;
    if !has_update_flags(args) {
        return Err(CfdError::message(
            "usage: cfd entry update <id> [--start <time>] [--end <time> | --duration <d>] [fields...]",
        ));
    }
    let user = client.get_current_user()?;
    let existing = client.get_time_entry(workspace_id, entry_id)?;
    let payload = build_time_entry_update_payload(args, config_state, &existing)?;
    let warning = find_overlaps_for_payload(
        client,
        workspace_id,
        &user.id,
        Some(entry_id),
        payload["start"].as_str().unwrap(),
        payload["end"].as_str().unwrap(),
    )?;
    maybe_confirm_overlap(&warning, args.yes)?;
    let entry = client.update_time_entry(workspace_id, entry_id, &payload)?;
    println!("{}", format_resource_id(&entry.id));
    Ok(())
}

fn delete_entry<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let entry_id = args
        .positional
        .first()
        .ok_or_else(|| CfdError::message("usage: cfd entry delete <id>"))?;
    client.delete_time_entry(workspace_id, entry_id)
}

fn list_entry_texts<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    validate_columns_with_format(args)?;
    let columns = parse_entry_text_columns(args.flags.get("columns").map(String::as_str))?;
    let project_id = resolve_entry_text_project(args, config_state)?;
    let user = client.get_current_user()?;
    let entries = client.list_time_entries(
        workspace_id,
        &user.id,
        &EntryFilters {
            project: Some(project_id.clone()),
            ..EntryFilters::default()
        },
    )?;
    let items = collect_entry_texts(entries, &project_id);

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&items)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!("{}", format_entry_text_items(&items, &args.output)?);
            } else {
                println!("{}", format_entry_text_table(&items, &columns));
            }
        }
    }

    Ok(())
}

fn build_time_entry_payload(
    args: &ParsedArgs,
    config_state: &StoredConfig,
) -> Result<serde_json::Value, CfdError> {
    let start = args.flags.get("start").map(String::as_str);
    let start = start.ok_or_else(|| {
        CfdError::message("usage: cfd entry add --start <time> (--end <time> | --duration <d>)")
    })?;
    let end = args.flags.get("end").map(String::as_str);
    let duration_value = args.flags.get("duration").map(String::as_str);

    if end.is_some() == duration_value.is_some() {
        return Err(CfdError::message(
            "use exactly one of --end <time> or --duration <d>",
        ));
    }

    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let start = datetime::resolve_and_round_timestamp("start", start, rounding)?;
    let start_dt = chrono::DateTime::parse_from_rfc3339(&start)
        .map_err(|_| CfdError::message(format!("invalid start: {start}")))?;
    let end_dt = match (end, duration_value) {
        (Some(end), None) => {
            let end = datetime::resolve_and_round_timestamp("end", end, rounding)?;
            chrono::DateTime::parse_from_rfc3339(&end)
                .map_err(|_| CfdError::message(format!("invalid end: {end}")))?
        }
        (None, Some(duration)) => {
            let parsed = duration::parse_duration(duration)?;
            let calculated_end = (start_dt + parsed).to_rfc3339();
            chrono::DateTime::parse_from_rfc3339(&datetime::round_timestamp(
                &calculated_end,
                rounding,
            )?)
            .map_err(|_| CfdError::message("invalid calculated end"))?
        }
        _ => unreachable!(),
    };

    if end_dt <= start_dt {
        return Err(CfdError::message(
            "end must be after start; if this came from rounding, retry with --no-rounding",
        ));
    }

    let mut payload = serde_json::json!({
        "start": start,
        "end": end_dt.to_rfc3339(),
    });

    apply_explicit_entry_fields(args, &mut payload);

    Ok(payload)
}

fn build_time_entry_update_payload(
    args: &ParsedArgs,
    config_state: &StoredConfig,
    existing: &TimeEntry,
) -> Result<serde_json::Value, CfdError> {
    if !has_update_flags(args) {
        return Err(CfdError::message(
            "usage: cfd entry update <id> [--start <time>] [--end <time> | --duration <d>] [fields...]",
        ));
    }

    let end = args.flags.get("end").map(String::as_str);
    let duration_value = args.flags.get("duration").map(String::as_str);
    if end.is_some() && duration_value.is_some() {
        return Err(CfdError::message(
            "use at most one of --end <time> or --duration <d>",
        ));
    }

    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let start = match args.flags.get("start").map(String::as_str) {
        Some(start) => datetime::resolve_and_round_existing_timestamp(
            "start",
            start,
            Some(&existing.time_interval.start),
            rounding,
        )?,
        None => existing.time_interval.start.clone(),
    };
    let start_dt = chrono::DateTime::parse_from_rfc3339(&start)
        .map_err(|_| CfdError::message(format!("invalid start: {start}")))?;

    let end = match (end, duration_value) {
        (Some(end), None) => datetime::resolve_and_round_existing_timestamp(
            "end",
            end,
            existing.time_interval.end.as_deref(),
            rounding,
        )?,
        (None, Some(duration)) => {
            let parsed = duration::parse_duration(duration)?;
            let calculated_end = (start_dt + parsed).to_rfc3339();
            datetime::round_timestamp(&calculated_end, rounding)?
        }
        (None, None) => existing.time_interval.end.clone().ok_or_else(|| {
            CfdError::message(
                "entry update requires an end time for running entries; use --end <time> or --duration <d>",
            )
        })?,
        (Some(_), Some(_)) => unreachable!(),
    };
    let end_dt = chrono::DateTime::parse_from_rfc3339(&end)
        .map_err(|_| CfdError::message(format!("invalid end: {end}")))?;

    if end_dt <= start_dt {
        return Err(CfdError::message(
            "end must be after start; if this came from rounding, retry with --no-rounding",
        ));
    }

    let mut payload = serde_json::json!({
        "start": start,
        "end": end,
    });
    apply_existing_entry_fields(existing, &mut payload);
    apply_explicit_entry_fields(args, &mut payload);

    Ok(payload)
}

fn has_update_flags(args: &ParsedArgs) -> bool {
    [
        "start",
        "end",
        "duration",
        "project",
        "task",
        "tag",
        "description",
    ]
    .iter()
    .any(|flag| args.flags.contains_key(*flag))
}

fn apply_existing_entry_fields(entry: &TimeEntry, payload: &mut serde_json::Value) {
    payload["description"] = serde_json::Value::String(entry.description.clone());
    if let Some(project_id) = &entry.project_id {
        payload["projectId"] = serde_json::Value::String(project_id.clone());
    }
    if let Some(task_id) = &entry.task_id {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if !entry.tag_ids.is_empty() {
        payload["tagIds"] = serde_json::Value::Array(
            entry
                .tag_ids
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
    }
}

fn apply_explicit_entry_fields(args: &ParsedArgs, payload: &mut serde_json::Value) {
    if let Some(description) = args.flags.get("description") {
        payload["description"] = serde_json::Value::String(description.clone());
    }
    if let Some(project_id) = args.flags.get("project") {
        payload["projectId"] = serde_json::Value::String(project_id.clone());
    }
    if let Some(task_id) = args.flags.get("task") {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if let Some(tag_id) = args.flags.get("tag") {
        payload["tagIds"] =
            serde_json::Value::Array(vec![serde_json::Value::String(tag_id.clone())]);
    }
}

fn find_overlaps_for_payload<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
    exclude_id: Option<&str>,
    start: &str,
    end: &str,
) -> Result<Option<crate::types::OverlapWarning>, CfdError> {
    let entries = client.list_time_entries(workspace_id, user_id, &EntryFilters::default())?;
    let overlapping_ids = find_overlaps(&entries, start, Some(end), exclude_id)?;
    if overlapping_ids.is_empty() {
        Ok(None)
    } else {
        Ok(Some(crate::types::OverlapWarning { overlapping_ids }))
    }
}

fn find_overlaps(
    entries: &[TimeEntry],
    start: &str,
    end: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Vec<String>, CfdError> {
    let start_dt = chrono::DateTime::parse_from_rfc3339(start)
        .map_err(|_| CfdError::message(format!("invalid start: {start}")))?;
    let end_dt = end
        .map(chrono::DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| CfdError::message(format!("invalid end: {}", end.unwrap_or_default())))?;

    let mut overlapping = Vec::new();

    for entry in entries {
        if exclude_id == Some(entry.id.as_str()) {
            continue;
        }

        let existing_start = chrono::DateTime::parse_from_rfc3339(&entry.time_interval.start)
            .map_err(|_| CfdError::message("invalid existing start"))?;
        let existing_end = entry
            .time_interval
            .end
            .as_deref()
            .map(chrono::DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|_| CfdError::message("invalid existing end"))?;

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

fn maybe_confirm_overlap(
    warning: &Option<crate::types::OverlapWarning>,
    yes: bool,
) -> Result<(), CfdError> {
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

fn filters_from_args(args: &ParsedArgs) -> Result<EntryFilters, CfdError> {
    Ok(EntryFilters {
        start: args
            .flags
            .get("start")
            .map(|value| datetime::resolve_list_datetime("start", value))
            .transpose()?,
        end: args
            .flags
            .get("end")
            .map(|value| datetime::resolve_list_datetime("end", value))
            .transpose()?,
        project: args.flags.get("project").cloned(),
        task: args.flags.get("task").cloned(),
        tags: args
            .flags
            .iter()
            .filter_map(|(key, value)| (key == "tag").then_some(value.clone()))
            .collect(),
        description: args.flags.get("text").cloned(),
    })
}

fn resolve_entry_text_project(
    args: &ParsedArgs,
    config_state: &StoredConfig,
) -> Result<String, CfdError> {
    let explicit_project = args.flags.get("project").map(String::as_str);
    config::resolve_project(explicit_project, config_state).map_err(|_| {
        CfdError::message("missing project; use --project <id> or cfd config set project <id>")
    })
}

fn collect_entry_texts(entries: Vec<TimeEntry>, project_id: &str) -> Vec<EntryTextItem> {
    let mut by_text = BTreeMap::<String, EntryTextItem>::new();

    for entry in entries
        .into_iter()
        .filter(|entry| entry.project_id.as_deref() == Some(project_id))
    {
        let trimmed = entry.description.trim();
        if trimmed.is_empty() {
            continue;
        }

        let start = entry.time_interval.start;
        by_text
            .entry(trimmed.to_owned())
            .and_modify(|item| {
                if start > item.last_used {
                    item.last_used = start.clone();
                }
                item.usage_count = Some(item.usage_count.unwrap_or(0) + 1);
            })
            .or_insert_with(|| EntryTextItem {
                text: trimmed.to_owned(),
                last_used: start,
                usage_count: Some(1),
            });
    }

    let mut items = by_text.into_values().collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.last_used
            .cmp(&a.last_used)
            .then_with(|| a.text.cmp(&b.text))
    });
    items
}

fn format_entry_text(
    entry: &TimeEntry,
    project_name: Option<&String>,
    output: &OutputOptions,
    columns: &[EntryColumn],
) -> String {
    if columns.is_empty() {
        format_entry_fields(
            entry,
            project_name,
            output,
            &[
                EntryColumn::Id,
                EntryColumn::Start,
                EntryColumn::End,
                EntryColumn::Duration,
                EntryColumn::Description,
                EntryColumn::Project,
                EntryColumn::ProjectName,
                EntryColumn::Task,
                EntryColumn::Tags,
            ],
        )
    } else {
        format_entry_fields(entry, project_name, output, columns)
    }
}

fn format_entry_table(
    entries: &[TimeEntry],
    project_names: &BTreeMap<String, String>,
    columns: &[EntryColumn],
) -> String {
    format_tsv_rows(
        &entries
            .iter()
            .map(|entry| {
                columns
                    .iter()
                    .map(|column| column.value(entry, project_names))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn format_entry_fields(
    entry: &TimeEntry,
    project_name: Option<&String>,
    output: &OutputOptions,
    columns: &[EntryColumn],
) -> String {
    let tags_joined = (!entry.tag_ids.is_empty()).then(|| entry.tag_ids.join(", "));
    let duration_display = entry
        .time_interval
        .duration
        .as_deref()
        .map(format_clockify_duration)
        .transpose()
        .ok()
        .flatten();
    let fields = columns
        .iter()
        .filter_map(|column| match column {
            EntryColumn::Id => Some(TextField {
                label: "id",
                value: &entry.id,
                is_meta: true,
            }),
            EntryColumn::Start => Some(TextField {
                label: "start",
                value: &entry.time_interval.start,
                is_meta: false,
            }),
            EntryColumn::End => Some(TextField {
                label: "end",
                value: entry.time_interval.end.as_deref().unwrap_or("-"),
                is_meta: false,
            }),
            EntryColumn::Duration => duration_display.as_deref().map(|value| TextField {
                label: "duration",
                value,
                is_meta: false,
            }),
            EntryColumn::Description => Some(TextField {
                label: "description",
                value: &entry.description,
                is_meta: false,
            }),
            EntryColumn::Project => entry.project_id.as_deref().map(|value| TextField {
                label: "projectId",
                value,
                is_meta: false,
            }),
            EntryColumn::ProjectName => project_name.map(|value| TextField {
                label: "project",
                value,
                is_meta: false,
            }),
            EntryColumn::Task => entry.task_id.as_deref().map(|value| TextField {
                label: "taskId",
                value,
                is_meta: false,
            }),
            EntryColumn::Tags => tags_joined.as_deref().map(|value| TextField {
                label: "tagIds",
                value,
                is_meta: false,
            }),
        })
        .collect::<Vec<_>>();

    format_text_fields(&fields, output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntrySort {
    Asc,
    Desc,
}

pub(crate) fn parse_entry_sort(value: Option<&str>, usage: &str) -> Result<EntrySort, CfdError> {
    match value {
        None => Ok(EntrySort::Asc),
        Some("asc") => Ok(EntrySort::Asc),
        Some("desc") => Ok(EntrySort::Desc),
        Some("true") => Err(CfdError::message(usage)),
        Some(other) => Err(CfdError::message(format!(
            "invalid sort: {other}; expected asc or desc"
        ))),
    }
}

pub(crate) fn sort_entries(
    entries: Vec<TimeEntry>,
    sort: EntrySort,
) -> Result<Vec<TimeEntry>, CfdError> {
    let mut keyed = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let start: DateTime<FixedOffset> =
                DateTime::parse_from_rfc3339(&entry.time_interval.start)
                    .map_err(|_| CfdError::message("invalid entry start"))?;
            Ok((start, index, entry))
        })
        .collect::<Result<Vec<_>, CfdError>>()?;

    keyed.sort_by(|a, b| match sort {
        EntrySort::Asc => a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)),
        EntrySort::Desc => b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)),
    });

    Ok(keyed.into_iter().map(|(_, _, entry)| entry).collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryColumn {
    Id,
    Start,
    End,
    Duration,
    Description,
    Project,
    ProjectName,
    Task,
    Tags,
}

impl EntryColumn {
    fn value(self, entry: &TimeEntry, project_names: &BTreeMap<String, String>) -> String {
        match self {
            EntryColumn::Id => entry.id.clone(),
            EntryColumn::Start => entry.time_interval.start.clone(),
            EntryColumn::End => entry
                .time_interval
                .end
                .clone()
                .unwrap_or_else(|| "-".into()),
            EntryColumn::Duration => entry
                .time_interval
                .duration
                .as_deref()
                .and_then(|value| format_clockify_duration(value).ok())
                .unwrap_or_default(),
            EntryColumn::Description => entry.description.clone(),
            EntryColumn::Project => entry.project_id.clone().unwrap_or_default(),
            EntryColumn::ProjectName => entry
                .project_id
                .as_deref()
                .and_then(|id| project_names.get(id))
                .cloned()
                .unwrap_or_default(),
            EntryColumn::Task => entry.task_id.clone().unwrap_or_default(),
            EntryColumn::Tags => entry.tag_ids.join(","),
        }
    }
}

fn parse_entry_columns(value: Option<&str>) -> Result<Vec<EntryColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd entry <list|get> ... --columns <id,start,end,duration,description,projectId,projectName,...>",
        |item| match item {
            "id" => Ok(EntryColumn::Id),
            "start" => Ok(EntryColumn::Start),
            "end" => Ok(EntryColumn::End),
            "duration" => Ok(EntryColumn::Duration),
            "description" => Ok(EntryColumn::Description),
            "project" | "projectId" => Ok(EntryColumn::Project),
            "projectName" => Ok(EntryColumn::ProjectName),
            "task" => Ok(EntryColumn::Task),
            "tags" => Ok(EntryColumn::Tags),
            other => Err(CfdError::message(format!("invalid entry column: {other}"))),
        },
    )
}

fn load_project_names_for_entries<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    entries: &[TimeEntry],
) -> Result<BTreeMap<String, String>, CfdError> {
    let project_ids = entries
        .iter()
        .filter_map(|entry| entry.project_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    if project_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let projects = client.list_projects(workspace_id)?;
    Ok(projects
        .into_iter()
        .filter(|project| project_ids.contains(project.id.as_str()))
        .map(|project| (project.id, project.name))
        .collect())
}

fn format_clockify_duration(value: &str) -> Result<String, CfdError> {
    if !value.starts_with("PT") {
        return Err(CfdError::message(format!("invalid duration: {value}")));
    }

    let mut hours = 0_i64;
    let mut minutes = 0_i64;
    let mut seconds = 0_i64;
    let mut digits = String::new();

    for char in value[2..].chars() {
        if char.is_ascii_digit() {
            digits.push(char);
            continue;
        }

        let amount = digits
            .parse::<i64>()
            .map_err(|_| CfdError::message(format!("invalid duration: {value}")))?;
        digits.clear();

        match char {
            'H' => hours = amount,
            'M' => minutes = amount,
            'S' => seconds = amount,
            _ => return Err(CfdError::message(format!("invalid duration: {value}"))),
        }
    }

    if !digits.is_empty() {
        return Err(CfdError::message(format!("invalid duration: {value}")));
    }

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }

    Ok(parts.join(""))
}

fn validate_entry_view_flags(args: &ParsedArgs) -> Result<(), CfdError> {
    validate_columns_with_format(args)
}

fn format_entry_text_table(items: &[EntryTextItem], columns: &[EntryTextColumn]) -> String {
    format_tsv_rows(
        &items
            .iter()
            .map(|item| {
                columns
                    .iter()
                    .map(|column| column.value(item))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_entry_text_columns(value: Option<&str>) -> Result<Vec<EntryTextColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd entry text list --columns <text,lastUsed,count,...>",
        |item| match item {
            "text" => Ok(EntryTextColumn::Text),
            "lastUsed" => Ok(EntryTextColumn::LastUsed),
            "count" => Ok(EntryTextColumn::Count),
            other => Err(CfdError::message(format!(
                "invalid entry text column: {other}"
            ))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryTextColumn {
    Text,
    LastUsed,
    Count,
}

impl EntryTextColumn {
    fn value(self, item: &EntryTextItem) -> String {
        match self {
            EntryTextColumn::Text => item.text.clone(),
            EntryTextColumn::LastUsed => item.last_used.clone(),
            EntryTextColumn::Count => item
                .usage_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::format::OutputOptions;
    use crate::types::TimeInterval;

    struct MockTransport {
        user_response: String,
        list_response: String,
        write_response: String,
        last_method: Rc<RefCell<Option<String>>>,
        last_body: Rc<RefCell<Option<String>>>,
    }

    impl MockTransport {
        fn new(
            user_response: &str,
            list_response: &str,
            write_response: &str,
        ) -> (
            Self,
            Rc<RefCell<Option<String>>>,
            Rc<RefCell<Option<String>>>,
        ) {
            let last_method = Rc::new(RefCell::new(None));
            let last_body = Rc::new(RefCell::new(None));
            (
                Self {
                    user_response: user_response.to_owned(),
                    list_response: list_response.to_owned(),
                    write_response: write_response.to_owned(),
                    last_method: Rc::clone(&last_method),
                    last_body: Rc::clone(&last_body),
                },
                last_method,
                last_body,
            )
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, url: &str, _api_key: &str) -> Result<String, CfdError> {
            self.last_method.replace(Some("GET".into()));
            if url.ends_with("/user") {
                Ok(self.user_response.clone())
            } else if url.contains("/time-entries/e1") {
                Ok(self.write_response.clone())
            } else {
                Ok(self.list_response.clone())
            }
        }

        fn post(&self, _url: &str, _api_key: &str, body: &str) -> Result<String, CfdError> {
            self.last_method.replace(Some("POST".into()));
            self.last_body.replace(Some(body.to_owned()));
            Ok(self.write_response.clone())
        }

        fn put(&self, _url: &str, _api_key: &str, body: &str) -> Result<String, CfdError> {
            self.last_method.replace(Some("PUT".into()));
            self.last_body.replace(Some(body.to_owned()));
            Ok(self.write_response.clone())
        }

        fn patch(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected patch"))
        }

        fn delete(&self, _url: &str, _api_key: &str) -> Result<(), CfdError> {
            self.last_method.replace(Some("DELETE".into()));
            Ok(())
        }
    }

    fn entry(id: &str, description: &str, start: &str) -> TimeEntry {
        TimeEntry {
            id: id.into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: None,
            task_id: None,
            tag_ids: vec![],
            description: description.into(),
            time_interval: TimeInterval {
                start: start.into(),
                end: None,
                duration: None,
            },
        }
    }

    fn closed_existing_entry() -> TimeEntry {
        TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: Some("t1".into()),
            tag_ids: vec!["tag1".into()],
            description: "Focus".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:00:00Z".into()),
                duration: None,
            },
        }
    }

    #[test]
    fn maps_text_flag_to_description_filter() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("list".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([
                ("text".into(), "focus".into()),
                ("start".into(), "2026-04-23T09:00:00Z".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let filters = filters_from_args(&args).unwrap();

        assert_eq!(filters.description.as_deref(), Some("focus"));
        assert_eq!(filters.start.as_deref(), Some("2026-04-23T09:00:00+00:00"));
    }

    #[test]
    fn entry_text_output_respects_no_meta() {
        let entry = TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: Some("t1".into()),
            tag_ids: vec!["tag1".into()],
            description: "Focus".into(),
            time_interval: crate::types::TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:00:00Z".into()),
                duration: Some("PT1H".into()),
            },
        };
        let project_name = "Project One".to_string();
        assert_eq!(
            format_entry_text(&entry, Some(&project_name), &OutputOptions::default(), &[]),
            "id: e1\nstart: 2026-04-23T09:00:00Z\nend: 2026-04-23T10:00:00Z\nduration: 1h\ndescription: Focus\nprojectId: p1\nproject: Project One\ntaskId: t1\ntagIds: tag1"
        );
        assert_eq!(
            format_entry_text(
                &entry,
                Some(&project_name),
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                },
                &[]
            ),
            "start: 2026-04-23T09:00:00Z\nend: 2026-04-23T10:00:00Z\nduration: 1h\ndescription: Focus\nprojectId: p1\nproject: Project One\ntaskId: t1\ntagIds: tag1"
        );
    }

    #[test]
    fn entry_columns_limit_text_output() {
        let entry = TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: None,
            tag_ids: vec![],
            description: "Focus".into(),
            time_interval: crate::types::TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:00:00Z".into()),
                duration: Some("PT1H".into()),
            },
        };
        assert_eq!(
            format_entry_table(
                &[entry],
                &BTreeMap::from([("p1".to_string(), "Project One".to_string())]),
                &[
                    EntryColumn::Start,
                    EntryColumn::End,
                    EntryColumn::Duration,
                    EntryColumn::Project,
                    EntryColumn::ProjectName,
                ],
            ),
            "2026-04-23T09:00:00Z\t2026-04-23T10:00:00Z\t1h\tp1\tProject One"
        );
    }

    #[test]
    fn bare_columns_is_rejected() {
        let error = parse_entry_columns(Some("true")).unwrap_err().to_string();

        assert!(error.contains("usage: cfd entry <list|get>"));
        assert!(error
            .contains("--columns <id,start,end,duration,description,projectId,projectName,...>"));
    }

    #[test]
    fn entry_sort_defaults_to_asc_and_accepts_values() {
        assert_eq!(parse_entry_sort(None, "usage").unwrap(), EntrySort::Asc);
        assert_eq!(
            parse_entry_sort(Some("asc"), "usage").unwrap(),
            EntrySort::Asc
        );
        assert_eq!(
            parse_entry_sort(Some("desc"), "usage").unwrap(),
            EntrySort::Desc
        );
    }

    #[test]
    fn entry_sort_rejects_bare_and_invalid_values() {
        let bare = parse_entry_sort(Some("true"), "usage: cfd entry list ... --sort <asc|desc>")
            .unwrap_err()
            .to_string();
        assert!(bare.contains("usage: cfd entry list"));

        let invalid = parse_entry_sort(Some("newest"), "usage")
            .unwrap_err()
            .to_string();
        assert!(invalid.contains("invalid sort: newest; expected asc or desc"));
    }

    #[test]
    fn sort_entries_orders_oldest_first_for_asc() {
        let sorted = sort_entries(
            vec![
                entry("e2", "Newest", "2026-04-27T11:00:00Z"),
                entry("e1", "Oldest", "2026-04-27T09:00:00Z"),
            ],
            EntrySort::Asc,
        )
        .unwrap();

        assert_eq!(sorted[0].id, "e1");
        assert_eq!(sorted[1].id, "e2");
    }

    #[test]
    fn sort_entries_orders_newest_first_for_desc() {
        let sorted = sort_entries(
            vec![
                entry("e1", "Oldest", "2026-04-27T09:00:00Z"),
                entry("e2", "Newest", "2026-04-27T11:00:00Z"),
            ],
            EntrySort::Desc,
        )
        .unwrap();

        assert_eq!(sorted[0].id, "e2");
        assert_eq!(sorted[1].id, "e1");
    }

    #[test]
    fn entry_columns_accept_duration_project_id_and_name() {
        let columns = parse_entry_columns(Some("start,duration,projectId,projectName")).unwrap();

        assert_eq!(
            columns,
            vec![
                EntryColumn::Start,
                EntryColumn::Duration,
                EntryColumn::Project,
                EntryColumn::ProjectName,
            ]
        );
    }

    #[test]
    fn project_alias_maps_to_project_id_column() {
        let columns = parse_entry_columns(Some("project")).unwrap();

        assert_eq!(columns, vec![EntryColumn::Project]);
    }

    #[test]
    fn clockify_duration_is_rendered_human_readably() {
        assert_eq!(format_clockify_duration("PT1H").unwrap(), "1h");
        assert_eq!(format_clockify_duration("PT33M").unwrap(), "33m");
        assert_eq!(format_clockify_duration("PT1H15M").unwrap(), "1h15m");
        assert_eq!(format_clockify_duration("PT45S").unwrap(), "45s");
        assert_eq!(format_clockify_duration("PT0S").unwrap(), "0s");
    }

    #[test]
    fn columns_and_format_are_mutually_exclusive() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("list".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([
                ("columns".into(), "start,end".into()),
                ("format".into(), "json".into()),
            ]),
            output: OutputOptions {
                format: OutputFormat::Json,
                no_meta: false,
            },
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = validate_entry_view_flags(&args).unwrap_err().to_string();

        assert!(error.contains("use either --columns <list> or --format"));
    }
    #[test]
    fn entry_text_columns_parse_and_render() {
        let columns = parse_entry_text_columns(Some("text,count")).unwrap();
        let rendered = format_entry_text_table(
            &[EntryTextItem {
                text: "Focus work".into(),
                last_used: "2026-04-24T10:00:00Z".into(),
                usage_count: None,
            }],
            &columns,
        );

        assert_eq!(rendered, "Focus work\t");
    }

    #[test]
    fn entry_text_columns_require_value() {
        let error = parse_entry_text_columns(Some("true"))
            .unwrap_err()
            .to_string();

        assert!(error.contains("usage: cfd entry text list --columns <text,lastUsed,count,...>"));
    }

    #[test]
    fn collect_entry_texts_trims_dedupes_and_sorts() {
        let items = collect_entry_texts(
            vec![
                TimeEntry {
                    id: "e1".into(),
                    workspace_id: "w1".into(),
                    user_id: Some("u1".into()),
                    project_id: Some("p1".into()),
                    task_id: None,
                    tag_ids: vec![],
                    description: "  Focus work  ".into(),
                    time_interval: crate::types::TimeInterval {
                        start: "2026-04-23T11:00:00Z".into(),
                        end: None,
                        duration: None,
                    },
                },
                TimeEntry {
                    id: "e2".into(),
                    workspace_id: "w1".into(),
                    user_id: Some("u1".into()),
                    project_id: Some("p1".into()),
                    task_id: None,
                    tag_ids: vec![],
                    description: "Focus work".into(),
                    time_interval: crate::types::TimeInterval {
                        start: "2026-04-23T12:00:00Z".into(),
                        end: None,
                        duration: None,
                    },
                },
                TimeEntry {
                    id: "e3".into(),
                    workspace_id: "w1".into(),
                    user_id: Some("u1".into()),
                    project_id: Some("p1".into()),
                    task_id: None,
                    tag_ids: vec![],
                    description: "   ".into(),
                    time_interval: crate::types::TimeInterval {
                        start: "2026-04-23T13:00:00Z".into(),
                        end: None,
                        duration: None,
                    },
                },
                TimeEntry {
                    id: "e4".into(),
                    workspace_id: "w1".into(),
                    user_id: Some("u1".into()),
                    project_id: Some("p2".into()),
                    task_id: None,
                    tag_ids: vec![],
                    description: "Other project".into(),
                    time_interval: crate::types::TimeInterval {
                        start: "2026-04-23T14:00:00Z".into(),
                        end: None,
                        duration: None,
                    },
                },
            ],
            "p1",
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "Focus work");
        assert_eq!(items[0].last_used, "2026-04-23T12:00:00Z");
        assert_eq!(items[0].usage_count, Some(2));
    }

    #[test]
    fn entry_text_project_resolution_prefers_flag_then_config() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("text".into()),
            subaction: Some("list".into()),
            positional: Vec::new(),
            flags: std::collections::HashMap::from([("project".into(), "flag-project".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let config = StoredConfig {
            project: Some("stored-project".into()),
            ..StoredConfig::default()
        };

        assert_eq!(
            resolve_entry_text_project(&args, &config).unwrap(),
            "flag-project"
        );
        assert_eq!(
            resolve_entry_text_project(
                &ParsedArgs {
                    flags: Default::default(),
                    ..args.clone()
                },
                &config
            )
            .unwrap(),
            "stored-project"
        );
    }

    #[test]
    fn entry_text_project_resolution_errors_when_missing() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("text".into()),
            subaction: Some("list".into()),
            positional: Vec::new(),
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = resolve_entry_text_project(&args, &StoredConfig::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("cfd config set project <id>"));
    }

    #[test]
    fn build_payload_supports_duration_and_description() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("add".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T09:00:00Z".into()),
                ("duration".into(), "90m".into()),
                ("description".into(), "Focus".into()),
                ("project".into(), "p1".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let payload = build_time_entry_payload(&args, &StoredConfig::default()).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:00:00+00:00");
        assert_eq!(payload["end"], "2026-04-23T10:30:00+00:00");
        assert_eq!(payload["description"], "Focus");
        assert_eq!(payload["projectId"], "p1");
    }

    #[test]
    fn update_payload_preserves_existing_entry_fields() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T09:15:00Z".into()),
                ("end".into(), "2026-04-23T10:15:00Z".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();
        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:15:00+00:00");
        assert_eq!(payload["end"], "2026-04-23T10:15:00+00:00");
        assert_eq!(payload["description"], "Focus");
        assert_eq!(payload["projectId"], "p1");
        assert_eq!(payload["taskId"], "t1");
        assert_eq!(payload["tagIds"][0], "tag1");
    }

    #[test]
    fn update_payload_uses_existing_start_when_start_is_omitted() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("end".into(), "2026-04-23T10:30:00Z".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:00:00Z");
        assert_eq!(payload["end"], "2026-04-23T10:30:00+00:00");
    }

    #[test]
    fn update_payload_supports_duration_without_start() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("duration".into(), "2h".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:00:00Z");
        assert_eq!(payload["end"], "2026-04-23T11:00:00+00:00");
    }

    #[test]
    fn update_payload_supports_duration_with_new_start() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T10:00:00Z".into()),
                ("duration".into(), "2h".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T10:00:00+00:00");
        assert_eq!(payload["end"], "2026-04-23T12:00:00+00:00");
    }

    #[test]
    fn update_payload_adjusts_existing_end_with_bare_relative_value() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("end".into(), "-5m".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:00:00Z");
        assert_eq!(payload["end"], "2026-04-23T09:55:00+00:00");
    }

    #[test]
    fn update_payload_adjusts_existing_start_with_bare_relative_value() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("start".into(), "+10m".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:10:00+00:00");
        assert_eq!(payload["end"], "2026-04-23T10:00:00Z");
    }

    #[test]
    fn update_payload_uses_adjusted_start_for_duration() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([
                ("start".into(), "+10m".into()),
                ("duration".into(), "1h".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:10:00+00:00");
        assert_eq!(payload["end"], "2026-04-23T10:10:00+00:00");
    }

    #[test]
    fn update_payload_rejects_bare_relative_end_for_running_entry() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("end".into(), "-5m".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = entry("e1", "Focus", "2026-04-23T09:00:00Z");

        let error = build_time_entry_update_payload(&args, &StoredConfig::default(), &existing)
            .unwrap_err()
            .to_string();

        assert!(error.contains("cannot adjust missing end time"));
        assert!(error.contains("--end now-5m"));
    }

    #[test]
    fn update_payload_allows_metadata_only_update() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([(
                "description".into(),
                "Focus updated".into(),
            )]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let payload =
            build_time_entry_update_payload(&args, &StoredConfig::default(), &existing).unwrap();

        assert_eq!(payload["start"], "2026-04-23T09:00:00Z");
        assert_eq!(payload["end"], "2026-04-23T10:00:00Z");
        assert_eq!(payload["description"], "Focus updated");
        assert_eq!(payload["projectId"], "p1");
        assert_eq!(payload["taskId"], "t1");
        assert_eq!(payload["tagIds"][0], "tag1");
    }

    #[test]
    fn build_payload_rejects_invalid_arg_combinations() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("add".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T09:00:00Z".into()),
                ("end".into(), "2026-04-23T10:00:00Z".into()),
                ("duration".into(), "30m".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = build_time_entry_payload(&args, &StoredConfig::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("exactly one"));
    }

    #[test]
    fn update_payload_rejects_noop_update() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let error = build_time_entry_update_payload(&args, &StoredConfig::default(), &existing)
            .unwrap_err()
            .to_string();

        assert!(error.contains("entry update <id>"));
    }

    #[test]
    fn update_payload_rejects_end_before_existing_start() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([("end".into(), "2026-04-23T08:00:00Z".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = closed_existing_entry();

        let error = build_time_entry_update_payload(&args, &StoredConfig::default(), &existing)
            .unwrap_err()
            .to_string();

        assert!(error.contains("end must be after start"));
    }

    #[test]
    fn update_payload_rejects_metadata_only_update_for_running_entry() {
        let args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([(
                "description".into(),
                "Focus updated".into(),
            )]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let existing = entry("e1", "Focus", "2026-04-23T09:00:00Z");

        let error = build_time_entry_update_payload(&args, &StoredConfig::default(), &existing)
            .unwrap_err()
            .to_string();

        assert!(error.contains("requires an end time for running entries"));
    }

    #[test]
    fn add_update_and_delete_use_expected_http_paths() {
        let response = serde_json::to_string(&TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: None,
            tag_ids: vec![],
            description: "Focus".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:00:00Z".into()),
                duration: None,
            },
        })
        .unwrap();
        let user_json = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
        let empty_list = "[]";
        let (transport, last_method, _) = MockTransport::new(user_json, empty_list, &response);
        let client = ClockifyClient::new("secret".into(), transport);

        let add_args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("add".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T09:00:00Z".into()),
                ("end".into(), "2026-04-23T10:00:00Z".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        execute(&client, &add_args, "w1", &StoredConfig::default()).unwrap();
        assert_eq!(last_method.borrow().as_deref(), Some("POST"));

        let update_args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("update".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: std::collections::HashMap::from([
                ("start".into(), "2026-04-23T09:00:00Z".into()),
                ("end".into(), "2026-04-23T10:00:00Z".into()),
            ]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        execute(&client, &update_args, "w1", &StoredConfig::default()).unwrap();
        assert_eq!(last_method.borrow().as_deref(), Some("PUT"));

        let delete_args = ParsedArgs {
            resource: Some("entry".into()),
            action: Some("delete".into()),
            subaction: None,
            positional: vec!["e1".into()],
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        execute(&client, &delete_args, "w1", &StoredConfig::default()).unwrap();
        assert_eq!(last_method.borrow().as_deref(), Some("DELETE"));
    }
}
