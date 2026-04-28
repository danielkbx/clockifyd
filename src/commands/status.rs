use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::datetime::{self, WeekStart};
use crate::error::CfdError;
use crate::format::{format_json, OutputFormat};
use crate::types::{EntryFilters, TimeEntry};

const USAGE: &str = "usage: cfd status [--week-start monday|sunday]";
const SUMMARY_HEADERS: [&str; 4] = ["Project", "Task", "Description", "Duration"];

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    if args.flags.contains_key("columns") {
        return Err(CfdError::message("cfd status does not support --columns"));
    }
    if !args.positional.is_empty() || args.action.is_some() {
        return Err(CfdError::message(USAGE));
    }

    let week_start = parse_week_start(args.flags.get("week-start").map(String::as_str))?;
    let (today_start, today_end) = datetime::local_today_bounds()?;
    let (week_start_at, week_end) = datetime::local_week_bounds(week_start)?;
    let now = Utc::now();

    let user = client.get_current_user()?;
    let timers = client
        .get_current_timers(workspace_id)?
        .into_iter()
        .filter(|entry| entry.user_id.as_deref() == Some(user.id.as_str()))
        .collect::<Vec<_>>();
    let today_entries = client.list_time_entries(
        workspace_id,
        &user.id,
        &EntryFilters {
            start: Some(today_start.clone()),
            end: Some(today_end.clone()),
            ..EntryFilters::default()
        },
    )?;
    let week_entries = client.list_time_entries(
        workspace_id,
        &user.id,
        &EntryFilters {
            start: Some(week_start_at.clone()),
            end: Some(week_end.clone()),
            ..EntryFilters::default()
        },
    )?;
    let project_names =
        load_project_names(client, workspace_id, &timers, &today_entries, &week_entries)?;

    let report = build_status_report(
        timers.first(),
        &today_entries,
        &week_entries,
        &project_names,
        StatusBounds {
            today_start,
            today_end,
            week_start,
            week_start_at,
            week_end,
        },
        now,
    )?;

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&report)?),
        OutputFormat::Text => println!("{}", render_status_text(&report)),
    }

    Ok(())
}

fn parse_week_start(value: Option<&str>) -> Result<WeekStart, CfdError> {
    match value.unwrap_or("monday") {
        "monday" => Ok(WeekStart::Monday),
        "sunday" => Ok(WeekStart::Sunday),
        _ => Err(CfdError::message(USAGE)),
    }
}

fn load_project_names<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    timers: &[TimeEntry],
    today_entries: &[TimeEntry],
    week_entries: &[TimeEntry],
) -> Result<BTreeMap<String, String>, CfdError> {
    let project_ids = timers
        .iter()
        .chain(today_entries)
        .chain(week_entries)
        .filter_map(|entry| entry.project_id.as_deref())
        .collect::<BTreeSet<_>>();
    if project_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    Ok(client
        .list_projects(workspace_id)?
        .into_iter()
        .filter(|project| project_ids.contains(project.id.as_str()))
        .map(|project| (project.id, project.name))
        .collect())
}

#[derive(Debug, Clone)]
struct StatusBounds {
    today_start: String,
    today_end: String,
    week_start: WeekStart,
    week_start_at: String,
    week_end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusReport {
    timer: TimerStatus,
    today: SummaryStatus,
    week: WeekSummaryStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimerStatus {
    running: bool,
    entry: Option<TimerEntryStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimerEntryStatus {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    description: String,
    start: String,
    duration_seconds: i64,
    duration: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SummaryStatus {
    start: String,
    end: String,
    groups: Vec<SummaryGroup>,
    total_seconds: i64,
    total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WeekSummaryStatus {
    week_start: String,
    start: String,
    end: String,
    groups: Vec<SummaryGroup>,
    total_seconds: i64,
    total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SummaryGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    description: String,
    duration_seconds: i64,
    duration: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    project_name: String,
    project_id: Option<String>,
    task_display: String,
    task_id: Option<String>,
    description_display: String,
    description: String,
}

fn build_status_report(
    timer: Option<&TimeEntry>,
    today_entries: &[TimeEntry],
    week_entries: &[TimeEntry],
    project_names: &BTreeMap<String, String>,
    bounds: StatusBounds,
    now: DateTime<Utc>,
) -> Result<StatusReport, CfdError> {
    Ok(StatusReport {
        timer: TimerStatus {
            running: timer.is_some(),
            entry: timer
                .map(|entry| timer_entry_status(entry, project_names, now))
                .transpose()?,
        },
        today: summary_status(
            &bounds.today_start,
            &bounds.today_end,
            today_entries,
            project_names,
            now,
        )?,
        week: WeekSummaryStatus {
            week_start: week_start_label(bounds.week_start).into(),
            start: bounds.week_start_at.clone(),
            end: bounds.week_end.clone(),
            groups: summary_groups(week_entries, project_names, now)?,
            total_seconds: total_duration_seconds(week_entries, now)?,
            total: format_duration(chrono::Duration::seconds(total_duration_seconds(
                week_entries,
                now,
            )?)),
        },
    })
}

fn timer_entry_status(
    entry: &TimeEntry,
    project_names: &BTreeMap<String, String>,
    now: DateTime<Utc>,
) -> Result<TimerEntryStatus, CfdError> {
    let duration_seconds = entry_duration(entry, now)?.num_seconds();
    Ok(TimerEntryStatus {
        id: entry.id.clone(),
        project_id: entry.project_id.clone(),
        project_name: entry
            .project_id
            .as_deref()
            .and_then(|id| project_names.get(id))
            .cloned(),
        task_id: entry.task_id.clone(),
        description: entry.description.clone(),
        start: entry.time_interval.start.clone(),
        duration_seconds,
        duration: format_duration(chrono::Duration::seconds(duration_seconds)),
    })
}

fn summary_status(
    start: &str,
    end: &str,
    entries: &[TimeEntry],
    project_names: &BTreeMap<String, String>,
    now: DateTime<Utc>,
) -> Result<SummaryStatus, CfdError> {
    let total_seconds = total_duration_seconds(entries, now)?;
    Ok(SummaryStatus {
        start: start.into(),
        end: end.into(),
        groups: summary_groups(entries, project_names, now)?,
        total_seconds,
        total: format_duration(chrono::Duration::seconds(total_seconds)),
    })
}

fn summary_groups(
    entries: &[TimeEntry],
    project_names: &BTreeMap<String, String>,
    now: DateTime<Utc>,
) -> Result<Vec<SummaryGroup>, CfdError> {
    let mut durations = BTreeMap::<GroupKey, chrono::Duration>::new();

    for entry in entries {
        let key = group_key(entry, project_names);
        let duration = entry_duration(entry, now)?;
        durations
            .entry(key)
            .and_modify(|existing| *existing += duration)
            .or_insert(duration);
    }

    Ok(durations
        .into_iter()
        .map(|(key, duration)| {
            let duration_seconds = duration.num_seconds();
            SummaryGroup {
                project_id: key.project_id,
                project_name: (!key.project_name.is_empty()).then_some(key.project_name),
                task_id: key.task_id,
                description: key.description,
                duration_seconds,
                duration: format_duration(duration),
            }
        })
        .collect())
}

fn group_key(entry: &TimeEntry, project_names: &BTreeMap<String, String>) -> GroupKey {
    let project_name = entry
        .project_id
        .as_deref()
        .and_then(|id| project_names.get(id).cloned())
        .or_else(|| entry.project_id.clone())
        .unwrap_or_default();
    let task_display = display_optional(entry.task_id.as_deref()).to_owned();
    let description_display =
        display_optional((!entry.description.is_empty()).then_some(entry.description.as_str()))
            .to_owned();

    GroupKey {
        project_name,
        project_id: entry.project_id.clone(),
        task_display,
        task_id: entry.task_id.clone(),
        description_display,
        description: entry.description.clone(),
    }
}

fn total_duration_seconds(entries: &[TimeEntry], now: DateTime<Utc>) -> Result<i64, CfdError> {
    entries.iter().try_fold(0, |total, entry| {
        Ok(total + entry_duration(entry, now)?.num_seconds())
    })
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

fn render_status_text(report: &StatusReport) -> String {
    let mut out = String::new();
    out.push_str("Timer:\n");
    out.push_str(if report.timer.running {
        "  running: yes\n"
    } else {
        "  running: no\n"
    });
    let timer_rows = timer_summary_rows(report.timer.entry.as_ref());
    let today_rows = summary_rows(&report.today.groups);
    let week_rows = summary_rows(&report.week.groups);
    let widths = summary_table_widths(
        &timer_rows,
        &today_rows,
        &report.today.total,
        &week_rows,
        &report.week.total,
    );
    if report.timer.running {
        push_table_text(&mut out, &timer_rows, None, &widths);
    }

    out.push('\n');
    push_summary_text(&mut out, "Today", &today_rows, &report.today.total, &widths);
    out.push('\n');
    push_summary_text(&mut out, "Week", &week_rows, &report.week.total, &widths);
    out
}

fn timer_summary_rows(entry: Option<&TimerEntryStatus>) -> Vec<SummaryRow> {
    entry
        .map(|entry| SummaryRow {
            project: display_project(entry.project_name.as_deref(), entry.project_id.as_deref()),
            task: display_optional(entry.task_id.as_deref()).to_owned(),
            description: display_optional(
                (!entry.description.is_empty()).then_some(entry.description.as_str()),
            )
            .to_owned(),
            duration: entry.duration.clone(),
        })
        .into_iter()
        .collect()
}

fn summary_rows(groups: &[SummaryGroup]) -> Vec<SummaryRow> {
    groups
        .iter()
        .map(|group| SummaryRow {
            project: display_project(group.project_name.as_deref(), group.project_id.as_deref()),
            task: display_optional(group.task_id.as_deref()).to_owned(),
            description: display_optional(
                (!group.description.is_empty()).then_some(group.description.as_str()),
            )
            .to_owned(),
            duration: group.duration.clone(),
        })
        .collect()
}

fn push_summary_text(
    out: &mut String,
    title: &str,
    rows: &[SummaryRow],
    total: &str,
    widths: &[usize; 4],
) {
    out.push_str(title);
    out.push_str(":\n");
    push_table_text(out, rows, Some(total), widths);
}

fn push_table_text(
    out: &mut String,
    rows: &[SummaryRow],
    total: Option<&str>,
    widths: &[usize; 4],
) {
    let table = render_summary_table(rows, total, widths);
    for line in table.lines() {
        out.push_str("  ");
        out.push_str(line);
        out.push('\n');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryRow {
    project: String,
    task: String,
    description: String,
    duration: String,
}

impl SummaryRow {
    fn cells(&self) -> [&str; 4] {
        [&self.project, &self.task, &self.description, &self.duration]
    }
}

fn render_summary_table(rows: &[SummaryRow], total: Option<&str>, widths: &[usize; 4]) -> String {
    let separator = summary_table_separator(widths);
    let mut lines = Vec::new();
    lines.push(separator.clone());
    lines.push(summary_table_row(&SUMMARY_HEADERS, widths));
    lines.push(separator.clone());
    for row in rows {
        lines.push(summary_table_row(&row.cells(), widths));
    }
    if let Some(total) = total {
        lines.push(separator.clone());
        lines.push(summary_table_row(&["Total", "", "", total], widths));
    }
    lines.push(separator);
    lines.join("\n")
}

fn summary_table_widths(
    timer_rows: &[SummaryRow],
    today_rows: &[SummaryRow],
    today_total: &str,
    week_rows: &[SummaryRow],
    week_total: &str,
) -> [usize; 4] {
    let mut widths = SUMMARY_HEADERS.map(str::len);
    for row in timer_rows.iter().chain(today_rows).chain(week_rows) {
        for (index, value) in row.cells().iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    widths[0] = widths[0].max("Total".len());
    widths[3] = widths[3].max(today_total.len()).max(week_total.len());
    widths
}

fn summary_table_separator(widths: &[usize; 4]) -> String {
    let mut line = String::new();
    line.push('+');
    for width in widths {
        line.push_str(&"-".repeat(width + 2));
        line.push('+');
    }
    line
}

fn summary_table_row(cells: &[&str; 4], widths: &[usize; 4]) -> String {
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

fn display_project(project_name: Option<&str>, project_id: Option<&str>) -> String {
    project_name
        .or(project_id)
        .filter(|value| !value.is_empty())
        .unwrap_or("none")
        .to_owned()
}

fn display_optional(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("none")
}

fn week_start_label(week_start: WeekStart) -> &'static str {
    match week_start {
        WeekStart::Monday => "monday",
        WeekStart::Sunday => "sunday",
    }
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
    fn groups_entries_by_project_task_and_description() {
        let now = DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let groups = summary_groups(
            &[
                entry(
                    "e1",
                    Some("p1"),
                    Some("t1"),
                    "Planning",
                    "2026-04-28T09:00:00Z",
                    Some("2026-04-28T10:00:00Z"),
                ),
                entry(
                    "e2",
                    Some("p1"),
                    Some("t1"),
                    "Planning",
                    "2026-04-28T10:00:00Z",
                    Some("2026-04-28T10:30:00Z"),
                ),
                entry(
                    "e3",
                    Some("p1"),
                    Some("t1"),
                    "Review",
                    "2026-04-28T11:00:00Z",
                    Some("2026-04-28T11:15:00Z"),
                ),
            ],
            &BTreeMap::from([("p1".into(), "Project One".into())]),
            now,
        )
        .unwrap();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].description, "Planning");
        assert_eq!(groups[0].duration, "1h30m");
        assert_eq!(groups[1].description, "Review");
        assert_eq!(groups[1].duration, "15m");
    }

    #[test]
    fn running_entries_count_toward_summary() {
        let now = DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let groups = summary_groups(
            &[entry("e1", None, None, "", "2026-04-28T11:45:00Z", None)],
            &BTreeMap::new(),
            now,
        )
        .unwrap();

        assert_eq!(groups[0].duration, "15m");
    }

    #[test]
    fn duration_format_uses_compact_units() {
        assert_eq!(format_duration(chrono::Duration::seconds(45)), "45s");
        assert_eq!(format_duration(chrono::Duration::minutes(12)), "12m");
        assert_eq!(format_duration(chrono::Duration::minutes(65)), "1h5m");
        assert_eq!(format_duration(chrono::Duration::minutes(1575)), "26h15m");
    }

    #[test]
    fn renders_summary_table_with_total_row() {
        let rendered = render_summary_table(
            &[SummaryRow {
                project: "Project One".into(),
                task: "t1".into(),
                description: "Planning".into(),
                duration: "1h30m".into(),
            }],
            Some("1h30m"),
            &summary_table_widths(
                &[],
                &[SummaryRow {
                    project: "Project One".into(),
                    task: "t1".into(),
                    description: "Planning".into(),
                    duration: "1h30m".into(),
                }],
                "1h30m",
                &[],
                "0s",
            ),
        );

        assert!(rendered.contains("| Project     | Task | Description | Duration |"));
        assert!(rendered.contains("| Project One | t1"));
        assert!(rendered.contains("| Planning"));
        assert!(rendered.contains("| 1h30m"));
        assert!(rendered.contains("| Total"));
    }

    #[test]
    fn summary_table_widths_are_shared_between_today_and_week() {
        let today_rows = vec![SummaryRow {
            project: "A".into(),
            task: "none".into(),
            description: "Short".into(),
            duration: "5m".into(),
        }];
        let week_rows = vec![SummaryRow {
            project: "Long Project Name".into(),
            task: "task-123".into(),
            description: "Longer description".into(),
            duration: "12h30m".into(),
        }];
        let widths = summary_table_widths(&[], &today_rows, "5m", &week_rows, "12h30m");

        let today = render_summary_table(&today_rows, Some("5m"), &widths);
        let week = render_summary_table(&week_rows, Some("12h30m"), &widths);

        assert_eq!(today.lines().next().unwrap(), week.lines().next().unwrap());
        assert!(today.contains("| A                 | none"));
        assert!(week.contains("| Long Project Name | task-123"));
    }

    #[test]
    fn summary_table_widths_include_running_timer() {
        let timer_rows = vec![SummaryRow {
            project: "Very Long Timer Project".into(),
            task: "none".into(),
            description: "Running".into(),
            duration: "1h".into(),
        }];
        let today_rows = vec![SummaryRow {
            project: "A".into(),
            task: "none".into(),
            description: "Short".into(),
            duration: "5m".into(),
        }];
        let week_rows = vec![SummaryRow {
            project: "B".into(),
            task: "none".into(),
            description: "Short".into(),
            duration: "10m".into(),
        }];
        let widths = summary_table_widths(&timer_rows, &today_rows, "5m", &week_rows, "10m");

        let timer = render_summary_table(&timer_rows, None, &widths);
        let today = render_summary_table(&today_rows, Some("5m"), &widths);
        let week = render_summary_table(&week_rows, Some("10m"), &widths);

        assert_eq!(timer.lines().next().unwrap(), today.lines().next().unwrap());
        assert_eq!(today.lines().next().unwrap(), week.lines().next().unwrap());
    }
}
