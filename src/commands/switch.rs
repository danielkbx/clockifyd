use chrono::Utc;
use serde::Serialize;

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::timer::{self, TimerStartFields};
use crate::config;
use crate::datetime;
use crate::error::CfdError;
use crate::format::{format_json, OutputFormat, OutputOptions};
use crate::types::{StoredConfig, StoredSwitch, StoredTimerFields, TimeEntry};

const USAGE: &str = "usage: cfd switch <current|start|stop>";

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("current") => current(client, args, workspace_id, config_state),
        Some("start") => start(client, args, workspace_id, config_state, None),
        Some("stop") => stop(client, args, workspace_id, config_state),
        _ => Err(CfdError::message(USAGE)),
    }
}

pub fn start_with_fields<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
    fields: TimerStartFields,
) -> Result<(), CfdError> {
    start(client, args, workspace_id, config_state, Some(fields))
}

pub fn stop_active_switch<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    stop(client, args, workspace_id, config_state)
}

fn start<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
    fields_override: Option<TimerStartFields>,
) -> Result<(), CfdError> {
    if args.flags.contains_key("description") {
        return Err(CfdError::message(
            "usage: cfd switch start [description] [--start <time>] [fields...] [--no-rounding] [-y]",
        ));
    }
    if fields_override.is_none() && args.positional.len() > 1 {
        return Err(CfdError::message(
            "usage: cfd switch start [description] [--start <time>] [fields...] [--no-rounding] [-y]",
        ));
    }
    if config_state.active_switch.is_some() {
        return Err(CfdError::message("switch already active"));
    }

    let user = client.get_current_user()?;
    let current = find_current_timer(client, workspace_id, &user.id)?;
    let return_to = timer_fields_from_entry(&current)?;
    let fields = match fields_override {
        Some(fields) => fields,
        None => {
            let explicit_project = args.flags.get("project").map(String::as_str);
            TimerStartFields {
                project_id: config::resolve_project(explicit_project, config_state).map_err(
                    |_| {
                        CfdError::message(
                            "missing project; use --project <id> or cfd config set project <id>",
                        )
                    },
                )?,
                task_id: args.flags.get("task").cloned(),
                tag_ids: args
                    .flags
                    .iter()
                    .filter_map(|(key, value)| (key == "tag").then_some(value.clone()))
                    .collect(),
                description: args.positional.first().cloned(),
            }
        }
    };

    let start = args
        .flags
        .get("start")
        .cloned()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let switch_at = datetime::resolve_and_round_timestamp("start", &start, rounding)?;
    let switch_at_dt = chrono::DateTime::parse_from_rfc3339(&switch_at)
        .map_err(|_| CfdError::message(format!("invalid start: {switch_at}")))?;
    let current_start = chrono::DateTime::parse_from_rfc3339(&current.time_interval.start)
        .map_err(|_| CfdError::message("invalid timer start"))?;
    if switch_at_dt <= current_start {
        return Err(CfdError::message(
            "switch start must be after current timer start; if this came from rounding, retry with --no-rounding",
        ));
    }

    let stopped = client.stop_timer(workspace_id, &user.id, &switch_at)?;
    let switched =
        client.create_time_entry(workspace_id, &timer_start_payload(&fields, &switch_at))?;

    let mut next_config = config_state.clone();
    next_config.active_switch = Some(StoredSwitch {
        workspace_id: workspace_id.to_owned(),
        user_id: user.id,
        original_entry_id: stopped.id,
        switched_entry_id: switched.id.clone(),
        switched_start: switch_at,
        return_start: Some(current.time_interval.start),
        return_to,
    });
    config::save_config(&next_config)?;

    println!("{}", switched.id);
    Ok(())
}

fn stop<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;
    let active_switch = config_state
        .active_switch
        .as_ref()
        .filter(|active_switch| {
            active_switch.workspace_id == workspace_id && active_switch.user_id == user.id
        })
        .ok_or_else(|| CfdError::message("no active switch"))?;
    let current = find_current_timer(client, workspace_id, &user.id)?;
    if current.id != active_switch.switched_entry_id {
        return Err(CfdError::message(
            "switch state is stale: current timer does not match switched timer",
        ));
    }

    let end = args
        .flags
        .get("end")
        .cloned()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let end = datetime::resolve_and_round_timestamp("end", &end, rounding)?;
    let end_dt = chrono::DateTime::parse_from_rfc3339(&end)
        .map_err(|_| CfdError::message(format!("invalid end: {end}")))?;
    let switched_start_dt = chrono::DateTime::parse_from_rfc3339(&active_switch.switched_start)
        .map_err(|_| CfdError::message("invalid switch start"))?;
    let resume_at = if end_dt > switched_start_dt {
        client.stop_timer(workspace_id, &user.id, &end)?;
        end
    } else {
        client.delete_time_entry(workspace_id, &active_switch.switched_entry_id)?;
        active_switch.switched_start.clone()
    };

    let restored = client.create_time_entry(
        workspace_id,
        &stored_timer_start_payload(&active_switch.return_to, &resume_at),
    )?;
    let mut next_config = config_state.clone();
    next_config.active_switch = None;
    config::save_config(&next_config)?;

    println!("{}", restored.id);
    Ok(())
}

fn current<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    if !args.positional.is_empty() {
        return Err(CfdError::message(
            "usage: cfd switch current [--format text|json|raw] [--no-meta]",
        ));
    }

    let Some(active_switch) = config_state
        .active_switch
        .as_ref()
        .filter(|active_switch| active_switch.workspace_id == workspace_id)
    else {
        return print_current_report(&SwitchCurrentReport::inactive(), &args.output);
    };

    let user = client.get_current_user()?;
    if active_switch.user_id != user.id {
        return print_current_report(&SwitchCurrentReport::inactive(), &args.output);
    }

    let current_timer = client
        .get_current_timers(workspace_id)?
        .into_iter()
        .find(|entry| entry.user_id.as_deref() == Some(user.id.as_str()))
        .ok_or_else(|| {
            CfdError::message("switch state is stale: current timer does not match switched timer")
        })?;

    if current_timer.id != active_switch.switched_entry_id {
        return Err(CfdError::message(
            "switch state is stale: current timer does not match switched timer",
        ));
    }

    let current_project_name = current_timer
        .project_id
        .as_deref()
        .and_then(|project_id| client.get_project(workspace_id, project_id).ok())
        .map(|project| project.name);
    let return_project_name = client
        .get_project(workspace_id, &active_switch.return_to.project_id)
        .ok()
        .map(|project| project.name);

    let report = SwitchCurrentReport::active(
        &current_timer,
        current_project_name,
        active_switch,
        return_project_name,
        Utc::now(),
    )?;
    print_current_report(&report, &args.output)
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

fn timer_fields_from_entry(entry: &TimeEntry) -> Result<StoredTimerFields, CfdError> {
    Ok(StoredTimerFields {
        project_id: entry
            .project_id
            .clone()
            .ok_or_else(|| CfdError::message("current timer has no project to return to"))?,
        task_id: entry.task_id.clone(),
        tag_ids: entry.tag_ids.clone(),
        description: (!entry.description.is_empty()).then(|| entry.description.clone()),
    })
}

fn timer_start_payload(fields: &TimerStartFields, start: &str) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "start": start,
        "projectId": fields.project_id,
    });
    if let Some(description) = &fields.description {
        payload["description"] = serde_json::Value::String(description.clone());
    }
    if let Some(task_id) = &fields.task_id {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if !fields.tag_ids.is_empty() {
        payload["tagIds"] = fields
            .tag_ids
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect();
    }
    payload
}

fn stored_timer_start_payload(fields: &StoredTimerFields, start: &str) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "start": start,
        "projectId": fields.project_id,
    });
    if let Some(description) = &fields.description {
        payload["description"] = serde_json::Value::String(description.clone());
    }
    if let Some(task_id) = &fields.task_id {
        payload["taskId"] = serde_json::Value::String(task_id.clone());
    }
    if !fields.tag_ids.is_empty() {
        payload["tagIds"] = fields
            .tag_ids
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect();
    }
    payload
}

fn print_current_report(
    report: &SwitchCurrentReport,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(report)?),
        OutputFormat::Text => print!("{}", render_current_text(report, output)),
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchCurrentReport {
    active: bool,
    current: Option<SwitchCurrentTimer>,
    returns_to: Option<SwitchReturnTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchCurrentTimer {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tag_ids: Vec<String>,
    description: String,
    start: String,
    duration_seconds: i64,
    duration: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchReturnTarget {
    original_entry_id: String,
    switched_at: String,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tag_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl SwitchCurrentReport {
    fn inactive() -> Self {
        Self {
            active: false,
            current: None,
            returns_to: None,
        }
    }

    fn active(
        current: &TimeEntry,
        current_project_name: Option<String>,
        active_switch: &StoredSwitch,
        return_project_name: Option<String>,
        now: chrono::DateTime<Utc>,
    ) -> Result<Self, CfdError> {
        Ok(Self {
            active: true,
            current: Some(current_timer_status(current, current_project_name, now)?),
            returns_to: Some(return_target_status(active_switch, return_project_name)),
        })
    }
}

fn current_timer_status(
    entry: &TimeEntry,
    project_name: Option<String>,
    now: chrono::DateTime<Utc>,
) -> Result<SwitchCurrentTimer, CfdError> {
    let start = chrono::DateTime::parse_from_rfc3339(&entry.time_interval.start)
        .map_err(|_| CfdError::message("invalid timer start"))?
        .with_timezone(&Utc);
    let duration = now.signed_duration_since(start);
    Ok(SwitchCurrentTimer {
        id: entry.id.clone(),
        project_id: entry.project_id.clone(),
        project_name,
        task_id: entry.task_id.clone(),
        tag_ids: entry.tag_ids.clone(),
        description: entry.description.clone(),
        start: entry.time_interval.start.clone(),
        duration_seconds: duration.num_seconds(),
        duration: timer::format_elapsed(duration),
    })
}

fn return_target_status(
    active_switch: &StoredSwitch,
    project_name: Option<String>,
) -> SwitchReturnTarget {
    let StoredTimerFields {
        project_id,
        task_id,
        tag_ids,
        description,
    } = &active_switch.return_to;
    SwitchReturnTarget {
        original_entry_id: active_switch.original_entry_id.clone(),
        switched_at: active_switch.switched_start.clone(),
        project_id: project_id.clone(),
        project_name,
        task_id: task_id.clone(),
        tag_ids: tag_ids.clone(),
        description: description.clone(),
    }
}

fn render_current_text(report: &SwitchCurrentReport, output: &OutputOptions) -> String {
    if !report.active {
        return "active: no\n".into();
    }

    let mut out = String::from("active: yes\n\n");
    if let Some(current) = &report.current {
        out.push_str("current:\n");
        if !output.no_meta {
            out.push_str(&format!("id: {}\n", current.id));
        }
        out.push_str(&format!("start: {}\n", current.start));
        out.push_str(&format!("duration: {}\n", current.duration));
        if !output.no_meta {
            out.push_str(&format!(
                "projectId: {}\n",
                current.project_id.as_deref().unwrap_or("")
            ));
        }
        out.push_str(&format!(
            "project: {}\n",
            current.project_name.as_deref().unwrap_or("")
        ));
        out.push_str(&format!(
            "taskId: {}\n",
            current.task_id.as_deref().unwrap_or("")
        ));
        out.push_str(&format!("description: {}\n", current.description));
    }

    out.push('\n');
    if let Some(returns_to) = &report.returns_to {
        out.push_str("returnsTo:\n");
        if !output.no_meta {
            out.push_str(&format!(
                "originalEntryId: {}\n",
                returns_to.original_entry_id
            ));
        }
        out.push_str(&format!("switchedAt: {}\n", returns_to.switched_at));
        if !output.no_meta {
            out.push_str(&format!("projectId: {}\n", returns_to.project_id));
        }
        out.push_str(&format!(
            "project: {}\n",
            returns_to.project_name.as_deref().unwrap_or("")
        ));
        out.push_str(&format!(
            "taskId: {}\n",
            returns_to.task_id.as_deref().unwrap_or("")
        ));
        out.push_str(&format!(
            "description: {}\n",
            returns_to.description.as_deref().unwrap_or("")
        ));
    }
    out
}
