use std::io::{self, BufRead, IsTerminal, Write};

use serde::Serialize;

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::timer::{self, TimerStartFields};
use crate::config;
use crate::error::CfdError;
use crate::format::{self, OutputOptions};
use crate::input;
use crate::types::{Project, StoredAlias, Task};

const BUILTIN_COMMANDS: &[&str] = &[
    "help",
    "login",
    "logout",
    "skill",
    "whoami",
    "workspace",
    "config",
    "project",
    "client",
    "tag",
    "task",
    "entry",
    "today",
    "timer",
    "completion",
    "alias",
];

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    opts: &OutputOptions,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("create") => create_alias(client, args, workspace_id),
        Some("list") => list_aliases(client, workspace_id, opts),
        Some("delete") => delete_alias(args),
        _ => Err(CfdError::message("usage: cfd alias <create|list|delete>")),
    }
}

pub fn execute_config_only(args: &ParsedArgs) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("delete") => delete_alias(args),
        _ => Err(CfdError::message("usage: cfd alias delete <alias> [-y]")),
    }
}

pub fn execute_runtime_start<T: HttpTransport>(
    client: &ClockifyClient<T>,
    alias_name: &str,
    alias: &StoredAlias,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &crate::types::StoredConfig,
) -> Result<(), CfdError> {
    for flag in ["project", "task", "description"] {
        if args.flags.contains_key(flag) {
            return Err(CfdError::message(format!(
                "cfd {alias_name} start does not accept --{flag}; update the alias instead"
            )));
        }
    }

    timer::start_timer_with_fields(
        client,
        args,
        workspace_id,
        config_state,
        TimerStartFields {
            project_id: alias.project.clone(),
            task_id: alias.task.clone(),
            tag_ids: Vec::new(),
            description: alias.description.clone(),
        },
    )
}

pub fn validate_alias_name(name: &str) -> Result<(), CfdError> {
    if !is_valid_alias_name(name) {
        return Err(CfdError::message(
            "alias names must match ^[a-z0-9][a-z0-9_-]*$",
        ));
    }
    if BUILTIN_COMMANDS.contains(&name) {
        return Err(CfdError::message(format!(
            "alias name conflicts with built-in command: {name}"
        )));
    }
    Ok(())
}

fn is_valid_alias_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn create_alias<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let name = args
        .positional
        .first()
        .ok_or_else(|| CfdError::message("usage: cfd alias create <alias>"))?;
    validate_alias_name(name)?;

    let mut config_state = config::get_config()?;
    let existing = config_state.aliases.get(name).cloned();
    let alias = build_alias(client, args, workspace_id, existing.as_ref())?;
    config_state.aliases.insert(name.clone(), alias);
    config::save_config(&config_state)?;
    println!("{name}");
    Ok(())
}

fn list_aliases<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    opts: &OutputOptions,
) -> Result<(), CfdError> {
    let config_state = config::get_config()?;
    let aliases = config_state
        .aliases
        .iter()
        .map(|(name, alias)| alias_output(client, workspace_id, name, alias))
        .collect::<Vec<_>>();

    match opts.format {
        format::OutputFormat::Json => println!("{}", format::format_json(&aliases)?),
        format::OutputFormat::Text => print!("{}", render_aliases_text(&aliases, opts)),
    }
    Ok(())
}

fn delete_alias(args: &ParsedArgs) -> Result<(), CfdError> {
    let name = args
        .positional
        .first()
        .ok_or_else(|| CfdError::message("usage: cfd alias delete <alias> [-y]"))?;
    validate_alias_name(name)?;

    let mut config_state = config::get_config()?;
    if !config_state.aliases.contains_key(name) {
        return Err(CfdError::message(format!("alias not found: {name}")));
    }
    if args.yes || input::confirm(&format!("Delete alias {name}?"))? {
        config_state.aliases.remove(name);
        config::save_config(&config_state)?;
        println!("{name}");
    }
    Ok(())
}

fn build_alias<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    existing: Option<&StoredAlias>,
) -> Result<StoredAlias, CfdError> {
    let interactive = io::stdin().is_terminal();

    let project = match args.flags.get("project") {
        Some(project) => {
            client.get_project(workspace_id, project)?;
            project.clone()
        }
        None if interactive => {
            prompt_project(client, workspace_id, existing.map(|a| a.project.as_str()))?
        }
        None => match existing {
            Some(existing) => existing.project.clone(),
            None => {
                return Err(CfdError::message(
                    "--project is required for new aliases in non-interactive mode",
                ))
            }
        },
    };

    let task = match args.flags.get("task").map(String::as_str) {
        Some("none") => None,
        Some(task) => {
            client.get_task(workspace_id, &project, task)?;
            Some(task.to_owned())
        }
        None if interactive => prompt_task(
            client,
            workspace_id,
            &project,
            existing.and_then(|a| a.task.as_deref()),
        )?,
        None => existing.and_then(|alias| alias.task.clone()),
    };

    let description = match args.flags.get("description").map(String::as_str) {
        Some("none") => None,
        Some(description) => Some(description.to_owned()),
        None if interactive => prompt_description(existing.and_then(|a| a.description.as_deref()))?,
        None => existing.and_then(|alias| alias.description.clone()),
    };

    Ok(StoredAlias {
        project,
        task,
        description,
    })
}

fn prompt_project<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    default: Option<&str>,
) -> Result<String, CfdError> {
    let mut projects = client.list_projects(workspace_id)?;
    projects.sort_by_key(|project| project.name.to_ascii_lowercase());
    let default_index = default.and_then(|id| projects.iter().position(|project| project.id == id));
    let index = prompt_choice("Project", &projects, default_index, |project| {
        project.name.clone()
    })?;
    Ok(projects[index].id.clone())
}

fn prompt_task<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    project_id: &str,
    default: Option<&str>,
) -> Result<Option<String>, CfdError> {
    let mut choices = vec![TaskChoice {
        id: None,
        label: "none".into(),
    }];
    let mut tasks = client.list_tasks(workspace_id, project_id)?;
    tasks.sort_by_key(|task| task.name.to_ascii_lowercase());
    choices.extend(tasks.into_iter().map(|task| TaskChoice {
        id: Some(task.id),
        label: task.name,
    }));
    let default_index = match default {
        Some(id) => choices
            .iter()
            .position(|choice| choice.id.as_deref() == Some(id)),
        None => Some(0),
    };
    let index = prompt_choice("Task", &choices, default_index, |choice| {
        choice.label.clone()
    })?;
    Ok(choices[index].id.clone())
}

fn prompt_description(default: Option<&str>) -> Result<Option<String>, CfdError> {
    eprint!("{}", render_text_prompt("Description", default));
    io::stderr().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.map(str::to_owned))
    } else if trimmed == "none" {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_owned()))
    }
}

fn prompt_choice<T, F>(
    label: &str,
    choices: &[T],
    default: Option<usize>,
    format: F,
) -> Result<usize, CfdError>
where
    F: Fn(&T) -> String,
{
    if choices.is_empty() {
        return Err(CfdError::message(format!("no {label} choices available")));
    }

    let default = default.filter(|idx| *idx < choices.len());
    eprint!("{}", render_choice_prompt(label, choices, default, &format));
    io::stderr().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default.ok_or_else(|| CfdError::message(format!("{label} selection is required")));
    }
    let number = trimmed
        .parse::<usize>()
        .map_err(|_| CfdError::message(format!("invalid {label} selection: {trimmed}")))?;
    if number == 0 || number > choices.len() {
        return Err(CfdError::message(format!(
            "invalid {label} selection: {trimmed}"
        )));
    }
    Ok(number - 1)
}

fn render_choice_prompt<T, F>(
    label: &str,
    choices: &[T],
    default: Option<usize>,
    format: &F,
) -> String
where
    F: Fn(&T) -> String,
{
    let default_label = default
        .filter(|idx| *idx < choices.len())
        .map(|idx| format(&choices[idx]));
    let mut out = String::new();
    out.push_str(label);
    out.push_str(":\n");
    for (idx, choice) in choices.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", idx + 1, format(choice)));
    }
    match default_label {
        Some(default_label) => out.push_str(&format!("Select {label} [{default_label}]: ")),
        None => out.push_str(&format!("Select {label}: ")),
    }
    out
}

fn render_text_prompt(label: &str, default: Option<&str>) -> String {
    match default {
        Some(default) => format!("{label} [{default}]: "),
        None => format!("{label}: "),
    }
}

struct TaskChoice {
    id: Option<String>,
    label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AliasOutput {
    alias: String,
    project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    project_resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_resolved: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_name: Option<String>,
}

fn alias_output<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    name: &str,
    alias: &StoredAlias,
) -> AliasOutput {
    let project = client.get_project(workspace_id, &alias.project).ok();
    let task = alias
        .task
        .as_ref()
        .and_then(|task| client.get_task(workspace_id, &alias.project, task).ok());

    AliasOutput {
        alias: name.to_owned(),
        project: alias.project.clone(),
        task: alias.task.clone(),
        description: alias.description.clone(),
        project_resolved: project.is_some(),
        task_resolved: alias.task.as_ref().map(|_| task.is_some()),
        project_name: project.map(|project| project.name),
        task_name: task.map(|task| task.name),
    }
}

fn render_aliases_text(aliases: &[AliasOutput], opts: &OutputOptions) -> String {
    let mut out = String::new();
    for (idx, alias) in aliases.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&alias.alias);
        out.push('\n');
        push_alias_line(
            &mut out,
            "project",
            alias.project_name.as_deref(),
            &alias.project,
            alias.project_resolved,
            opts,
        );
        match alias.task.as_deref() {
            Some(task) => push_alias_line(
                &mut out,
                "task",
                alias.task_name.as_deref(),
                task,
                alias.task_resolved.unwrap_or(false),
                opts,
            ),
            None => out.push_str("  task: none\n"),
        }
        out.push_str("  description: ");
        out.push_str(alias.description.as_deref().unwrap_or("none"));
        out.push('\n');
    }
    out
}

fn push_alias_line(
    out: &mut String,
    label: &str,
    display: Option<&str>,
    id: &str,
    resolved: bool,
    opts: &OutputOptions,
) {
    out.push_str("  ");
    out.push_str(label);
    out.push_str(": ");
    if let Some(display) = display {
        out.push_str(display);
        if !opts.no_meta {
            out.push_str(" (");
            out.push_str(id);
            out.push(')');
        }
    } else {
        out.push_str(id);
    }
    if !resolved {
        out.push_str(" [unresolved]");
    }
    out.push('\n');
}

#[allow(dead_code)]
fn _assert_alias_related_types(_: &Project, _: &Task) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_alias_names() {
        assert!(validate_alias_name("todo").is_ok());
        assert!(validate_alias_name("client-work_1").is_ok());
        assert!(validate_alias_name("123").is_ok());
        assert!(validate_alias_name("").is_err());
        assert!(validate_alias_name("Todo").is_err());
        assert!(validate_alias_name("client.work").is_err());
        assert!(validate_alias_name("timer").is_err());
        assert!(validate_alias_name("alias").is_err());
    }

    #[test]
    fn choice_prompt_without_default_matches_ytd_style() {
        let choices = vec!["Project One", "Project Two"];
        let rendered =
            render_choice_prompt("Project", &choices, None, &|choice| choice.to_string());

        assert!(rendered.contains("Project:\n"));
        assert!(rendered.contains("  1. Project One\n"));
        assert!(rendered.contains("Select Project: "));
        assert!(!rendered.contains("(default)"));
    }

    #[test]
    fn choice_prompt_with_default_shows_label_not_index() {
        let choices = vec!["Project One", "Project Two"];
        let rendered =
            render_choice_prompt("Project", &choices, Some(0), &|choice| choice.to_string());

        assert!(rendered.contains("Select Project [Project One]: "));
        assert!(!rendered.contains("Select Project [1]: "));
        assert!(!rendered.contains("(default)"));
    }

    #[test]
    fn task_prompt_includes_none_as_numbered_choice() {
        let choices = vec![
            TaskChoice {
                id: None,
                label: "none".into(),
            },
            TaskChoice {
                id: Some("t1".into()),
                label: "ABC-1: Implement feature".into(),
            },
        ];
        let rendered =
            render_choice_prompt("Task", &choices, Some(0), &|choice| choice.label.clone());

        assert!(rendered.contains("Task:\n"));
        assert!(rendered.contains("  1. none\n"));
        assert!(rendered.contains("  2. ABC-1: Implement feature\n"));
        assert!(rendered.contains("Select Task [none]: "));
        assert!(!rendered.contains("(default)"));
    }

    #[test]
    fn description_prompt_uses_value_default() {
        assert_eq!(
            render_text_prompt("Description", Some("Existing description")),
            "Description [Existing description]: "
        );
        assert_eq!(render_text_prompt("Description", None), "Description: ");
    }
}
