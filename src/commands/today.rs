use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Local, Utc};

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::datetime;
use crate::error::CfdError;
use crate::format::{format_json, OutputFormat};
use crate::types::{EntryFilters, TimeEntry};

const HEADERS: [&str; 5] = ["Project", "Task", "Description", "Time", "Duration"];

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    if args.flags.contains_key("columns") {
        return Err(CfdError::message(
            "cfd today does not support --columns; use cfd entry list --start today --end today --columns <list>",
        ));
    }

    let filters = EntryFilters {
        start: Some(datetime::resolve_list_datetime("start", "today")?),
        end: Some(datetime::resolve_list_datetime("end", "today")?),
        ..EntryFilters::default()
    };
    let user = client.get_current_user()?;
    let entries = client.list_time_entries(workspace_id, &user.id, &filters)?;

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&entries)?),
        OutputFormat::Text => {
            let project_names = load_project_names_for_entries(client, workspace_id, &entries)?;
            let rendered = render_today_entries(&entries, &project_names, Utc::now())?;
            println!("{rendered}");
        }
    }

    Ok(())
}

fn render_today_entries(
    entries: &[TimeEntry],
    project_names: &BTreeMap<String, String>,
    now: DateTime<Utc>,
) -> Result<String, CfdError> {
    let mut total = chrono::Duration::zero();
    let rows = entries
        .iter()
        .map(|entry| {
            let duration = entry_duration(entry, now)?;
            total += duration;
            Ok(TodayRow {
                project: entry
                    .project_id
                    .as_deref()
                    .and_then(|id| project_names.get(id))
                    .cloned()
                    .unwrap_or_default(),
                task: entry.task_id.clone().unwrap_or_default(),
                description: entry.description.clone(),
                time: format_time_range(entry)?,
                duration: format_duration(duration),
            })
        })
        .collect::<Result<Vec<_>, CfdError>>()?;

    Ok(render_today_table(&rows, &format_duration(total)))
}

fn load_project_names_for_entries<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    entries: &[TimeEntry],
) -> Result<BTreeMap<String, String>, CfdError> {
    let project_ids = entries
        .iter()
        .filter_map(|entry| entry.project_id.as_deref())
        .collect::<BTreeSet<_>>();
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

fn format_time_range(entry: &TimeEntry) -> Result<String, CfdError> {
    let start = format_local_time(&entry.time_interval.start)?;
    let end = entry
        .time_interval
        .end
        .as_deref()
        .map(format_local_time)
        .transpose()?
        .unwrap_or_else(|| "now".into());

    Ok(format!("{start}-{end}"))
}

fn format_local_time(value: &str) -> Result<String, CfdError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| CfdError::message(format!("invalid timestamp: {value}")))?;
    Ok(parsed.with_timezone(&Local).format("%H:%M").to_string())
}

fn entry_duration(entry: &TimeEntry, now: DateTime<Utc>) -> Result<chrono::Duration, CfdError> {
    let start = DateTime::parse_from_rfc3339(&entry.time_interval.start)
        .map_err(|_| CfdError::message("invalid entry start"))?
        .with_timezone(&Utc);
    let end = entry
        .time_interval
        .end
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| CfdError::message("invalid entry end"))?
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(now);
    let duration = end - start;

    if duration < chrono::Duration::zero() {
        Ok(chrono::Duration::zero())
    } else {
        Ok(duration)
    }
}

fn format_duration(duration: chrono::Duration) -> String {
    let mut seconds = duration.num_seconds().max(0);
    let hours = seconds / 3600;
    seconds %= 3600;
    let minutes = seconds / 60;
    seconds %= 60;

    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h{minutes}m")
        } else {
            format!("{hours}h")
        }
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{seconds}s")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodayRow {
    project: String,
    task: String,
    description: String,
    time: String,
    duration: String,
}

fn render_today_table(rows: &[TodayRow], total: &str) -> String {
    let mut widths = HEADERS.map(str::len);
    for row in rows {
        for (index, value) in row.cells().iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    widths[0] = widths[0].max("Total".len());
    widths[4] = widths[4].max(total.len());

    let separator = table_separator(&widths);
    let mut lines = Vec::new();
    lines.push(separator.clone());
    lines.push(table_row(&HEADERS, &widths));
    lines.push(separator.clone());
    for row in rows {
        lines.push(table_row(&row.cells(), &widths));
    }
    lines.push(separator.clone());
    lines.push(table_row(&["Total", "", "", "", total], &widths));
    lines.push(separator);
    lines.join("\n")
}

impl TodayRow {
    fn cells(&self) -> [&str; 5] {
        [
            &self.project,
            &self.task,
            &self.description,
            &self.time,
            &self.duration,
        ]
    }
}

fn table_separator(widths: &[usize; 5]) -> String {
    let mut line = String::new();
    line.push('+');
    for width in widths {
        line.push_str(&"-".repeat(width + 2));
        line.push('+');
    }
    line
}

fn table_row(cells: &[&str; 5], widths: &[usize; 5]) -> String {
    let mut line = String::new();
    line.push('|');
    for (cell, width) in cells.iter().zip(widths) {
        line.push(' ');
        line.push_str(&format!("{cell:<width$}"));
        line.push(' ');
        line.push('|');
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TimeInterval;

    fn entry(
        id: &str,
        project_id: Option<&str>,
        task_id: Option<&str>,
        description: &str,
        start: &str,
        end: Option<&str>,
    ) -> TimeEntry {
        TimeEntry {
            id: id.into(),
            workspace_id: "w1".into(),
            user_id: Some("u1".into()),
            project_id: project_id.map(str::to_owned),
            task_id: task_id.map(str::to_owned),
            tag_ids: vec![],
            description: description.into(),
            time_interval: TimeInterval {
                start: start.into(),
                end: end.map(str::to_owned),
                duration: None,
            },
        }
    }

    #[test]
    fn renders_table_with_expected_header_order_and_total() {
        let table = render_today_table(
            &[TodayRow {
                project: "Project One".into(),
                task: "t1".into(),
                description: "Planning".into(),
                time: "09:00-10:15".into(),
                duration: "1h15m".into(),
            }],
            "1h15m",
        );

        assert!(table.contains("| Project     | Task | Description | Time        | Duration |"));
        assert!(table.contains("| Project One | t1   | Planning    | 09:00-10:15 | 1h15m    |"));
        assert!(table.contains("| Total       |      |             |             | 1h15m    |"));
    }

    #[test]
    fn maps_entries_to_project_task_description_time_and_duration() {
        let now = DateTime::parse_from_rfc3339("2026-04-27T10:15:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rendered = render_today_entries(
            &[entry(
                "e1",
                Some("p1"),
                Some("t1"),
                "Planning",
                "2026-04-27T09:00:00Z",
                Some("2026-04-27T10:15:00Z"),
            )],
            &BTreeMap::from([("p1".into(), "Project One".into())]),
            now,
        )
        .unwrap();

        assert!(rendered.contains("Project One"));
        assert!(rendered.contains("t1"));
        assert!(rendered.contains("Planning"));
        assert!(rendered.contains("1h15m"));
    }

    #[test]
    fn running_entries_show_now_and_count_toward_total() {
        let now = DateTime::parse_from_rfc3339("2026-04-27T10:15:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rendered = render_today_entries(
            &[entry(
                "e1",
                None,
                None,
                "Review PR",
                "2026-04-27T10:00:00Z",
                None,
            )],
            &BTreeMap::new(),
            now,
        )
        .unwrap();

        assert!(rendered.contains("-now"));
        assert!(rendered.contains("15m"));
        assert!(rendered.contains("| Total "));
    }

    #[test]
    fn empty_table_renders_zero_total() {
        let rendered = render_today_entries(&[], &BTreeMap::new(), Utc::now()).unwrap();

        assert!(rendered.contains("| Project | Task | Description | Time | Duration |"));
        assert!(rendered.contains("| Total   |      |             |      | 0s       |"));
    }

    #[test]
    fn invalid_timestamps_are_rejected() {
        let error = entry_duration(&entry("e1", None, None, "", "not-a-date", None), Utc::now())
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid entry start"));
    }

    #[test]
    fn duration_format_uses_compact_units() {
        assert_eq!(format_duration(chrono::Duration::seconds(45)), "45s");
        assert_eq!(format_duration(chrono::Duration::minutes(12)), "12m");
        assert_eq!(format_duration(chrono::Duration::minutes(65)), "1h5m");
        assert_eq!(format_duration(chrono::Duration::minutes(1575)), "26h15m");
    }
}
