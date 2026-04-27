use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::config;
use crate::datetime;
use crate::error::CfdError;
use crate::format::{
    format_json, format_resource_id, format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::input;
use crate::types::{EntryFilters, OverlapWarning, StoredConfig, TimeEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimerStartFields {
    pub project_id: String,
    pub task_id: Option<String>,
    pub description: Option<String>,
}

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("current") => current_timer(client, args, workspace_id),
        Some("start") => start_timer(client, args, workspace_id, config_state),
        Some("stop") => stop_timer(client, args, workspace_id, config_state),
        _ => Err(CfdError::message("usage: cfd timer <current|start|stop>")),
    }
}

fn current_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;
    let timer = find_current_timer(client, workspace_id, &user.id)?;
    print_timer(client, workspace_id, &timer, &args.output)
}

fn start_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    if args.flags.contains_key("description") {
        return Err(CfdError::message(
            "usage: cfd timer start [description] [--start <iso>] [fields...] [--no-rounding]",
        ));
    }
    if args.positional.len() > 1 {
        return Err(CfdError::message(
            "usage: cfd timer start [description] [--start <iso>] [fields...] [--no-rounding]",
        ));
    }
    let explicit_project = args.flags.get("project").map(String::as_str);
    let project_id = config::resolve_project(explicit_project, config_state).map_err(|_| {
        CfdError::message("missing project; use --project <id> or cfd config set project <id>")
    })?;
    let fields = TimerStartFields {
        project_id,
        task_id: args.flags.get("task").cloned(),
        description: args.positional.first().cloned(),
    };
    start_timer_with_fields(client, args, workspace_id, config_state, fields)
}

pub(crate) fn start_timer_with_fields<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
    fields: TimerStartFields,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;
    if find_current_timer_optional(client, workspace_id, &user.id)?.is_some() {
        return Err(CfdError::message("timer already running"));
    }

    let start = args
        .flags
        .get("start")
        .cloned()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let start = datetime::round_timestamp(&start, rounding)?;
    let _ = chrono::DateTime::parse_from_rfc3339(&start)
        .map_err(|_| CfdError::message(format!("invalid start: {start}")))?;

    let warning = find_overlaps(client, workspace_id, &user.id, &start, None, None)?;
    maybe_confirm_overlap(&warning, args.yes)?;

    let mut payload = serde_json::json!({
        "start": start,
        "projectId": fields.project_id,
    });
    if let Some(description) = fields.description {
        payload["description"] = serde_json::Value::String(description);
    }
    if let Some(task_id) = fields.task_id {
        payload["taskId"] = serde_json::Value::String(task_id);
    }

    let entry = client.create_time_entry(workspace_id, &payload)?;
    println!("{}", format_resource_id(&entry.id));
    Ok(())
}

fn stop_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    let user = client.get_current_user()?;
    let current = find_current_timer(client, workspace_id, &user.id)?;

    let end = args
        .flags
        .get("end")
        .cloned()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let rounding = config::resolve_rounding(args.no_rounding, config_state)?;
    let end = datetime::round_timestamp(&end, rounding)?;
    let end_dt = chrono::DateTime::parse_from_rfc3339(&end)
        .map_err(|_| CfdError::message(format!("invalid end: {end}")))?;
    let start_dt = chrono::DateTime::parse_from_rfc3339(&current.time_interval.start)
        .map_err(|_| CfdError::message("invalid timer start"))?;
    if end_dt <= start_dt {
        return Err(CfdError::message(
            "end must be after start; if this came from rounding, retry with --no-rounding",
        ));
    }

    let warning = find_overlaps(
        client,
        workspace_id,
        &user.id,
        &current.time_interval.start,
        Some(&end),
        Some(current.id.as_str()),
    )?;
    maybe_confirm_overlap(&warning, args.yes)?;

    let entry = client.stop_timer(workspace_id, &user.id, &end)?;
    print_timer(client, workspace_id, &entry, &args.output)
}

fn find_current_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
) -> Result<TimeEntry, CfdError> {
    find_current_timer_optional(client, workspace_id, user_id)?
        .ok_or_else(|| CfdError::message("no running timer"))
}

fn find_current_timer_optional<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    user_id: &str,
) -> Result<Option<TimeEntry>, CfdError> {
    let timers = client.get_current_timers(workspace_id)?;
    Ok(timers
        .into_iter()
        .find(|entry| entry.user_id.as_deref() == Some(user_id)))
}

fn print_timer<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    entry: &TimeEntry,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    let project_name = entry
        .project_id
        .as_deref()
        .map(|project_id| client.get_project(workspace_id, project_id))
        .transpose()?
        .map(|project| project.name);

    match output.format {
        OutputFormat::Json => println!("{}", format_json(entry)?),
        OutputFormat::Text => println!(
            "{}",
            format_timer_text(entry, project_name.as_deref(), output, chrono::Utc::now())?
        ),
    }

    Ok(())
}

fn format_timer_text(
    entry: &TimeEntry,
    project_name: Option<&str>,
    output: &OutputOptions,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<String, CfdError> {
    let start = chrono::DateTime::parse_from_rfc3339(&entry.time_interval.start)
        .map_err(|_| CfdError::message("invalid timer start"))?
        .with_timezone(&chrono::Utc);
    let elapsed = if let Some(end) = entry.time_interval.end.as_deref() {
        let end = chrono::DateTime::parse_from_rfc3339(end)
            .map_err(|_| CfdError::message("invalid timer end"))?
            .with_timezone(&chrono::Utc);
        format_elapsed(end.signed_duration_since(start))
    } else {
        format_elapsed(now.signed_duration_since(start))
    };

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
                label: "duration",
                value: &elapsed,
                is_meta: false,
            },
            TextField {
                label: "projectId",
                value: entry.project_id.as_deref().unwrap_or(""),
                is_meta: false,
            },
            TextField {
                label: "project",
                value: project_name.unwrap_or(""),
                is_meta: false,
            },
            TextField {
                label: "description",
                value: &entry.description,
                is_meta: false,
            },
        ],
        output,
    ))
}

fn format_elapsed(duration: chrono::Duration) -> String {
    let seconds = duration.num_seconds();
    let negative = seconds < 0;
    let seconds = seconds.abs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if secs > 0 || parts.is_empty() {
        parts.push(format!("{secs}s"));
    }

    let rendered = parts.join("");
    if negative {
        format!("-{rendered}")
    } else {
        rendered
    }
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
    let start_dt = chrono::DateTime::parse_from_rfc3339(start)
        .map_err(|_| CfdError::message(format!("invalid start: {start}")))?;
    let end_dt = end
        .map(chrono::DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| CfdError::message("invalid end"))?;

    let mut overlapping_ids = Vec::new();
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
            overlapping_ids.push(entry.id);
        }
    }

    if overlapping_ids.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OverlapWarning { overlapping_ids }))
    }
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::format::OutputOptions;
    use crate::types::TimeInterval;

    struct MockTransport {
        user_response: String,
        timer_response: String,
        write_response: String,
        method: Rc<RefCell<Option<String>>>,
    }

    impl MockTransport {
        fn new(
            user_response: &str,
            timer_response: &str,
            write_response: &str,
        ) -> (Self, Rc<RefCell<Option<String>>>) {
            let method = Rc::new(RefCell::new(None));
            (
                Self {
                    user_response: user_response.to_owned(),
                    timer_response: timer_response.to_owned(),
                    write_response: write_response.to_owned(),
                    method: Rc::clone(&method),
                },
                method,
            )
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, url: &str, _api_key: &str) -> Result<String, CfdError> {
            self.method.replace(Some("GET".into()));
            if url.ends_with("/user") {
                Ok(self.user_response.clone())
            } else {
                Ok(self.timer_response.clone())
            }
        }

        fn post(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            self.method.replace(Some("POST".into()));
            Ok(self.write_response.clone())
        }

        fn put(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected put"))
        }

        fn patch(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            self.method.replace(Some("PATCH".into()));
            Ok(self.write_response.clone())
        }

        fn delete(&self, _url: &str, _api_key: &str) -> Result<(), CfdError> {
            Err(CfdError::message("unexpected delete"))
        }
    }

    #[test]
    fn timer_text_output_respects_no_meta() {
        let entry = TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: None,
            tag_ids: vec![],
            description: "Run".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: None,
                duration: None,
            },
        };
        let now = chrono::DateTime::parse_from_rfc3339("2026-04-23T10:02:03Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert_eq!(
            format_timer_text(&entry, Some("Project One"), &OutputOptions::default(), now).unwrap(),
            "id: e1\nstart: 2026-04-23T09:00:00Z\nduration: 1h2m3s\nprojectId: p1\nproject: Project One\ndescription: Run"
        );
        assert_eq!(
            format_timer_text(
                &entry,
                Some("Project One"),
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                },
                now,
            )
            .unwrap(),
            "start: 2026-04-23T09:00:00Z\nduration: 1h2m3s\nprojectId: p1\nproject: Project One\ndescription: Run"
        );
    }

    #[test]
    fn format_elapsed_handles_small_and_negative_durations() {
        assert_eq!(format_elapsed(chrono::Duration::seconds(0)), "0s");
        assert_eq!(format_elapsed(chrono::Duration::seconds(59)), "59s");
        assert_eq!(format_elapsed(chrono::Duration::seconds(60)), "1m");
        assert_eq!(format_elapsed(chrono::Duration::seconds(3661)), "1h1m1s");
        assert_eq!(format_elapsed(chrono::Duration::seconds(-5)), "-5s");
        assert_eq!(format_elapsed(chrono::Duration::seconds(-3661)), "-1h1m1s");
    }

    #[test]
    fn timer_text_uses_end_for_stopped_entries() {
        let entry = TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: Some("p1".into()),
            task_id: None,
            tag_ids: vec![],
            description: "Run".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:02:03Z".into()),
                duration: None,
            },
        };
        let now = chrono::DateTime::parse_from_rfc3339("2026-04-23T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert_eq!(
            format_timer_text(&entry, Some("Project One"), &OutputOptions::default(), now)
                .unwrap(),
            "id: e1\nstart: 2026-04-23T09:00:00Z\nduration: 1h2m3s\nprojectId: p1\nproject: Project One\ndescription: Run"
        );
    }

    #[test]
    fn current_and_stop_validate_timer_state() {
        let user_json = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
        let (transport, _) = MockTransport::new(user_json, "[]", "{}");
        let client = ClockifyClient::new("secret".into(), transport);
        let error = find_current_timer(&client, "w1", "u1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("no running timer"));
    }

    #[test]
    fn start_uses_post_when_no_timer_is_running() {
        let user_json = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
        let entry_json = serde_json::to_string(&TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: None,
            task_id: None,
            tag_ids: vec![],
            description: "Run".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: None,
                duration: None,
            },
        })
        .unwrap();
        let (transport, method) = MockTransport::new(user_json, "[]", &entry_json);
        let client = ClockifyClient::new("secret".into(), transport);
        let start_args = ParsedArgs {
            resource: Some("timer".into()),
            action: Some("start".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([(
                "start".into(),
                "2026-04-23T09:00:00Z".into(),
            )]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        let config = StoredConfig {
            project: Some("p1".into()),
            ..StoredConfig::default()
        };
        start_timer(&client, &start_args, "w1", &config).unwrap();
        assert_eq!(method.borrow().as_deref(), Some("POST"));
    }

    #[test]
    fn start_requires_project_from_flag_or_config() {
        let user_json = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
        let (transport, _) = MockTransport::new(user_json, "[]", "{}");
        let client = ClockifyClient::new("secret".into(), transport);
        let start_args = ParsedArgs {
            resource: Some("timer".into()),
            action: Some("start".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([(
                "start".into(),
                "2026-04-23T09:00:00Z".into(),
            )]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = start_timer(&client, &start_args, "w1", &StoredConfig::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("missing project"));
        assert!(error.contains("cfd config set project <id>"));
    }

    #[test]
    fn stop_uses_patch_when_timer_is_running() {
        let user_json = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
        let stopped_entry_json = serde_json::to_string(&TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: None,
            task_id: None,
            tag_ids: vec![],
            description: "Run".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: Some("2026-04-23T10:00:00Z".into()),
                duration: None,
            },
        })
        .unwrap();
        let running_timers = serde_json::to_string(&vec![TimeEntry {
            id: "e1".into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: None,
            task_id: None,
            tag_ids: vec![],
            description: "Run".into(),
            time_interval: TimeInterval {
                start: "2026-04-23T09:00:00Z".into(),
                end: None,
                duration: None,
            },
        }])
        .unwrap();
        let (transport, method) =
            MockTransport::new(user_json, &running_timers, &stopped_entry_json);
        let client = ClockifyClient::new("secret".into(), transport);
        let stop_args = ParsedArgs {
            resource: Some("timer".into()),
            action: Some("stop".into()),
            subaction: None,
            positional: Vec::new(),
            flags: std::collections::HashMap::from([("end".into(), "2026-04-23T10:00:00Z".into())]),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };
        stop_timer(&client, &stop_args, "w1", &StoredConfig::default()).unwrap();
        assert_eq!(method.borrow().as_deref(), Some("PATCH"));
    }
}
