use crate::args::ParsedArgs;
use crate::error::CfdError;
use crate::types::{Project, Workspace};

const SKILL_USAGE: &str =
    "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillScope {
    Brief,
    Standard,
    Full,
}

impl SkillScope {
    fn parse(value: Option<&str>) -> Result<Self, CfdError> {
        match value {
            None => Ok(Self::Standard),
            Some("brief") => Ok(Self::Brief),
            Some("standard") => Ok(Self::Standard),
            Some("full") => Ok(Self::Full),
            Some(other) => Err(CfdError::message(format!(
                "Invalid scope: {other}. Expected one of: brief, standard, full"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillWorkspaceContext {
    id: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillProjectContext {
    id: String,
    name: String,
}

impl From<Workspace> for SkillWorkspaceContext {
    fn from(workspace: Workspace) -> Self {
        Self {
            id: workspace.id,
            name: workspace.name,
        }
    }
}

impl From<Project> for SkillProjectContext {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
        }
    }
}

pub fn workspace_ref(args: &ParsedArgs) -> Result<Option<&str>, CfdError> {
    match args.flags.get("workspace").map(String::as_str) {
        None => Ok(None),
        Some("true") | Some("") => Err(CfdError::message(SKILL_USAGE)),
        Some(workspace) => Ok(Some(workspace)),
    }
}

pub fn project_ref(args: &ParsedArgs) -> Result<Option<&str>, CfdError> {
    match args.flags.get("project").map(String::as_str) {
        None => Ok(None),
        Some("true") | Some("") => Err(CfdError::message(SKILL_USAGE)),
        Some(project) => Ok(Some(project)),
    }
}

pub fn validate(args: &ParsedArgs) -> Result<(), CfdError> {
    SkillScope::parse(args.flags.get("scope").map(String::as_str))?;
    let workspace = workspace_ref(args)?;
    let project = project_ref(args)?;

    if project.is_some() && workspace.is_none() {
        return Err(CfdError::message(SKILL_USAGE));
    }

    match args.flags.get("format").map(String::as_str) {
        None | Some("text" | "md") => Ok(()),
        Some(_) => Err(CfdError::message(
            "cfd skill only supports --format text or --format md",
        )),
    }
}

pub fn run(
    workspace: Option<SkillWorkspaceContext>,
    project: Option<SkillProjectContext>,
    args: &ParsedArgs,
) -> Result<(), CfdError> {
    validate(args)?;
    let scope = SkillScope::parse(args.flags.get("scope").map(String::as_str))?;
    println!(
        "{}",
        render_skill(scope, workspace.as_ref(), project.as_ref())
    );
    Ok(())
}

fn render_skill(
    scope: SkillScope,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) -> String {
    let mut out = String::new();
    push_frontmatter(&mut out, workspace, project);
    push_intro(&mut out);
    push_when_to_use(&mut out);
    push_version(&mut out, scope, workspace, project);
    if let Some(workspace) = workspace {
        push_workspace_context(&mut out, workspace);
    }
    if let Some(project) = project {
        push_project_context(&mut out, project);
    }
    push_help_guidance(&mut out);
    push_output_rules(&mut out);
    push_core_commands(&mut out, workspace, project);
    push_ids_and_scope(&mut out, workspace, project);
    push_safety(&mut out);

    if matches!(scope, SkillScope::Standard | SkillScope::Full) {
        push_workflow(&mut out);
        push_examples(&mut out, workspace, project);
        push_recipes(&mut out, workspace, project);
        push_rounding_and_overlaps(&mut out);
        push_work_logs_boundary(&mut out);
    }

    if matches!(scope, SkillScope::Full) {
        push_full_reference(&mut out);
    }

    out.trim_end().to_string()
}

fn push_frontmatter(
    out: &mut String,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    out.push_str("---\n");
    out.push_str(&format!("name: {}\n", skill_name(workspace, project)));
    match workspace {
        Some(workspace) => {
            out.push_str("description: >-\n");
            out.push_str(&format!(
                "  Use this skill when working with Clockify time tracking in workspace {} through the cfd CLI: tracking work time, timers, time entries, projects, clients, tasks, tags, defaults, and rounding. Use for Clockify time tracking records, not generic issue tracker work logs, unless the user explicitly wants Clockify/cfd time entries.\n",
                workspace.name
            ));
        }
        None => {
            out.push_str("description: >-\n");
            out.push_str("  Use this skill when working with Clockify time tracking through the cfd CLI: tracking work time, starting or stopping timers, creating, updating, deleting, listing, or inspecting Clockify time entries, and browsing Clockify workspaces, projects, clients, tasks, and tags. Use for Clockify time tracking records, not generic issue tracker work logs, unless the user explicitly wants Clockify/cfd time entries.\n");
        }
    }
    out.push_str("---\n\n");
}

fn skill_name(
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) -> String {
    let base = "clockify";
    let suffix_source = project
        .map(|project| (project.name.as_str(), project.id.as_str()))
        .or_else(|| workspace.map(|workspace| (workspace.name.as_str(), workspace.id.as_str())));
    let Some((name, id)) = suffix_source else {
        return base.to_string();
    };
    let suffix = sanitize_skill_name_part(name)
        .or_else(|| sanitize_skill_name_part(id))
        .unwrap_or_default();
    if suffix.is_empty() {
        return base.to_string();
    }

    let mut name = format!("{base}-{suffix}");
    if name.len() > 64 {
        name.truncate(64);
        while name.ends_with('-') {
            name.pop();
        }
    }
    name
}

fn sanitize_skill_name_part(value: &str) -> Option<String> {
    let mut out = String::new();
    let mut last_was_hyphen = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_hyphen = false;
        } else if !last_was_hyphen && !out.is_empty() {
            out.push('-');
            last_was_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn push_intro(out: &mut String) {
    out.push_str("# cfd Clockify Time Tracking CLI\n\n");
    out.push_str("Use `cfd` to manage Clockify time tracking from a terminal: workspaces, projects, clients, tags, tasks, manual time entries, running timers, stored defaults, and rounding.\n\n");
}

fn push_when_to_use(out: &mut String) {
    out.push_str("## When To Use\n\n");
    out.push_str("- Use this for Clockify time tracking, timers, time entries, durations, workspace/project/task/tag metadata, and Clockify defaults.\n");
    out.push_str("- Use this when the user mentions `cfd`, Clockify, time entries, timers, tracking work time, or Clockify workspace/project/task context.\n");
    out.push_str("- Do not use this for generic YouTrack, Jira, GitHub, or issue tracker work logs unless the user explicitly asks for Clockify/cfd time entries.\n\n");
}

fn push_version(
    out: &mut String,
    scope: SkillScope,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    let version = env!("CARGO_PKG_VERSION");
    out.push_str("## Keeping This Skill Current\n\n");
    out.push_str(&format!("This skill was generated for cfd {version}.\n\n"));
    out.push_str("Before relying on this file, run:\n\n");
    out.push_str("```bash\ncfd --version\n```\n\n");
    out.push_str(&format!(
        "If the installed cfd version is newer than {version}, regenerate this skill with the same command shape:\n\n"
    ));
    out.push_str("```bash\n");
    out.push_str(&regeneration_command(scope, workspace, project));
    out.push_str("\n```\n\n");
}

fn regeneration_command(
    scope: SkillScope,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) -> String {
    match (workspace, project) {
        (Some(workspace), Some(project)) => format!(
            "cfd skill --workspace {} --project {} --scope {} > SKILL.md",
            workspace.id,
            project.id,
            scope.as_str()
        ),
        (Some(workspace), None) => format!(
            "cfd skill --workspace {} --scope {} > SKILL.md",
            workspace.id,
            scope.as_str()
        ),
        (None, _) => format!("cfd skill --scope {} > SKILL.md", scope.as_str()),
    }
}

fn push_workspace_context(out: &mut String, workspace: &SkillWorkspaceContext) {
    out.push_str("## Workspace Context\n\n");
    out.push_str("Default Clockify workspace for examples:\n");
    out.push_str(&format!("- Name: {}\n", workspace.name));
    out.push_str(&format!("- ID: {}\n\n", workspace.id));
}

fn push_project_context(out: &mut String, project: &SkillProjectContext) {
    out.push_str("## Project Context\n\n");
    out.push_str("Default Clockify project for examples:\n");
    out.push_str(&format!("- Name: {}\n", project.name));
    out.push_str(&format!("- ID: {}\n\n", project.id));
}

fn push_help_guidance(out: &mut String) {
    out.push_str("## Help Lookup\n\n");
    out.push_str("- Run `cfd help` to see available commands.\n");
    out.push_str(
        "- Run `cfd help <command>` or `cfd <command> help` before using an unfamiliar command.\n",
    );
    out.push_str("- Prefer help lookup over guessing required flags, date syntax, columns, or delete behavior.\n\n");
}

fn push_output_rules(out: &mut String) {
    out.push_str("## Output Rules\n\n");
    out.push_str("- Prefer `--format json` for list/get commands when extracting IDs, comparing data, or planning follow-up commands.\n");
    out.push_str("- Use `--columns <list>` for compact tab-separated inspection when a command supports it.\n");
    out.push_str("- Entry timeline outputs (`entry list`, `today`) support `--sort asc|desc` and sort by start time ascending by default; use `--sort desc` for newest first.\n");
    out.push_str("- `status` gives a computed timer/today/week overview; JSON/raw output returns grouped summary data, not raw time-entry arrays.\n");
    out.push_str("- Use text output for quick human-readable inspection.\n");
    out.push_str("- `--format raw` is a compatibility alias for JSON on normal cfd commands. `cfd skill` supports only `--format text` and `--format md`.\n\n");
}

fn push_core_commands(
    out: &mut String,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    let workspace_flag = workspace_flag(workspace);
    let project_id = project_id(project);
    out.push_str("## Core Time Tracking Commands\n\n");
    out.push_str("```bash\n");
    out.push_str("cfd workspace list --format json\n");
    out.push_str(&format!("cfd project list{workspace_flag} --format json\n"));
    out.push_str(&format!(
        "cfd task list{workspace_flag} --project {project_id} --format json\n"
    ));
    out.push_str(&format!(
        "cfd entry list{workspace_flag} --start today --end today --format json --sort asc\n"
    ));
    out.push_str(&format!("cfd status{workspace_flag} --format json\n"));
    out.push_str(&format!("cfd entry add{workspace_flag} --start <iso> --duration <duration> --project {project_id} --description \"<work>\"\n"));
    out.push_str(&format!(
        "cfd entry update{workspace_flag} <entry-id> --end <iso>\n"
    ));
    out.push_str(&format!(
        "cfd entry update{workspace_flag} <entry-id> --duration <duration>\n"
    ));
    out.push_str(&format!(
        "cfd entry update{workspace_flag} <entry-id> --description \"<work>\"\n"
    ));
    out.push_str(&format!(
        "cfd timer current{workspace_flag} --format json\n"
    ));
    out.push_str(&format!(
        "cfd timer start \"<work>\"{workspace_flag} --project {project_id}\n"
    ));
    out.push_str(&format!("cfd timer stop{workspace_flag}\n"));
    out.push_str(&format!("cfd timer resume -1{workspace_flag} -y\n"));
    out.push_str("```\n\n");
}

fn push_ids_and_scope(
    out: &mut String,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    out.push_str("## IDs And Scope\n\n");
    if let Some(workspace) = workspace {
        out.push_str(&format!(
            "- Use workspace `{}` for workspace-scoped examples unless the user gives a different workspace.\n",
            workspace.id
        ));
    } else {
        out.push_str("- Run `cfd workspace list --format json` and confirm the workspace when workspace scope is ambiguous.\n");
    }
    if let Some(project) = project {
        out.push_str(&format!(
            "- Use project `{}` for project-scoped examples unless the user gives a different project.\n",
            project.id
        ));
    }
    out.push_str("- Use IDs returned by JSON output for follow-up commands.\n");
    out.push_str("- `task get` requires both project ID and task ID.\n");
    out.push_str("- Entry fields accept `--project`, `--task`, `--tag`, and `--description`.\n");
    out.push_str("- Timer start accepts `--project`, `--task`, and `--tag`; pass the description as one quoted positional argument.\n");
    out.push_str("- Timer resume copies project, task, tags, and description from a recent entry; `-1` selects the newest entry.\n\n");
}

fn push_safety(out: &mut String) {
    out.push_str("## Safety\n\n");
    out.push_str("- Read current state before updating or deleting a time entry.\n");
    out.push_str("- Confirm destructive intent with the user before `entry delete`.\n");
    out.push_str("- Use `-y` only when deletion or overlap confirmation is explicitly intended.\n");
    out.push_str("- Never print, log, or expose Clockify API keys or credential files.\n\n");
}

fn push_workflow(out: &mut String) {
    out.push_str("## Recommended Agent Workflow\n\n");
    out.push_str("1. Run command help before unfamiliar syntax.\n");
    out.push_str("2. Resolve or confirm workspace first when missing or ambiguous.\n");
    out.push_str("3. Use JSON list/get commands to discover IDs.\n");
    out.push_str("4. Read current state before mutating entries or timers.\n");
    out.push_str("5. For destructive or overlapping changes, explain the exact target before continuing.\n\n");
}

fn push_examples(
    out: &mut String,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    let workspace_flag = workspace_flag(workspace);
    let project_id = project_id(project);
    out.push_str("## Examples\n\n");
    out.push_str("```bash\n");
    out.push_str(&format!(
        "cfd project list{workspace_flag} --columns id,name\n"
    ));
    out.push_str(&format!("cfd entry list{workspace_flag} --start today --end today --columns start,end,duration,description --sort asc\n"));
    out.push_str(&format!(
        "cfd entry text list{workspace_flag} --project {project_id} --columns text,lastUsed\n"
    ));
    out.push_str(&format!(
        "cfd timer start \"ABC-1: Implement feature\"{workspace_flag} --project {project_id}\n"
    ));
    out.push_str("```\n\n");
}

fn push_recipes(
    out: &mut String,
    workspace: Option<&SkillWorkspaceContext>,
    project: Option<&SkillProjectContext>,
) {
    let workspace_flag = workspace_flag(workspace);
    let project_id = project_id(project);
    out.push_str("## Common Recipes\n\n");
    out.push_str(&format!(
        "- Find today’s tracked time in chronological order: `cfd entry list{workspace_flag} --start today --end today --format json --sort asc`.\n"
    ));
    out.push_str(&format!(
        "- Check current timer plus today/week totals: `cfd status{workspace_flag}`.\n"
    ));
    out.push_str(&format!(
        "- Add a manual entry: `cfd entry add{workspace_flag} --start <iso> --duration 30m --project {project_id} --description \"<work>\"`.\n"
    ));
    out.push_str(&format!(
        "- Start a timer: `cfd timer start \"<work>\"{workspace_flag} --project {project_id}`.\n"
    ));
    out.push_str(&format!(
        "- Stop a timer: `cfd timer stop{workspace_flag}`.\n"
    ));
    out.push_str(&format!(
        "- Resume the newest prior entry: `cfd timer resume -1{workspace_flag}`.\n"
    ));
    out.push_str(&format!(
        "- Reuse prior descriptions: `cfd entry text list{workspace_flag} --project {project_id} --format json`.\n\n"
    ));
}

fn push_rounding_and_overlaps(out: &mut String) {
    out.push_str("## Rounding And Overlaps\n\n");
    out.push_str("- Rounding applies to `entry add`, `entry update`, `timer start`, `timer stop`, and `timer resume` unless `--no-rounding` is present.\n");
    out.push_str("- Active rounding resolves from `CFD_ROUNDING`, stored config, then `off`.\n");
    out.push_str("- Overlap warnings are not hard errors, but they require confirmation unless `-y` is present.\n");
    out.push_str("- `-y` skips the prompt, not overlap detection.\n\n");
}

fn push_work_logs_boundary(out: &mut String) {
    out.push_str("## Work Logs Boundary\n\n");
    out.push_str("- Clockify time entries are independent time tracking records.\n");
    out.push_str("- Issue tracker work logs, comments, or status updates belong in the issue tracker unless the user asks for Clockify time tracking.\n");
    out.push_str("- When the user says “log work,” clarify whether they mean Clockify time tracking or an issue tracker work log if context is ambiguous.\n\n");
}

fn push_full_reference(out: &mut String) {
    out.push_str("## Command Reference\n\n");
    out.push_str("```text\n");
    out.push_str("cfd help / cfd help <command> / cfd <command> help\n");
    out.push_str("cfd --version / cfd completion <bash|zsh|fish>\n");
    out.push_str("cfd login / logout / whoami\n");
    out.push_str("cfd workspace list|get\n");
    out.push_str(
        "cfd config / config interactive / config set|get|unset workspace|project|rounding\n",
    );
    out.push_str("cfd project list|get / client list|get / tag list|get\n");
    out.push_str("cfd task list|get|create\n");
    out.push_str("cfd entry list|get|add|update|delete / cfd entry text list\n");
    out.push_str("cfd today / cfd status\n");
    out.push_str("cfd timer current|start|stop\n");
    out.push_str("```\n\n");
    out.push_str("## Detailed Output And Input Rules\n\n");
    out.push_str(
        "- `--format json` is the stable machine-readable format for normal list/get commands.\n",
    );
    out.push_str(
        "- `--format raw` is accepted as a JSON alias for compatibility on normal commands.\n",
    );
    out.push_str("- `--columns` emits no header and one tab-separated row per item; it cannot be combined with `--format`.\n");
    out.push_str("- `entry list` and `today` support `--sort asc|desc`; default `asc` puts newest entries last.\n");
    out.push_str("- `status` groups today/week summaries by project, task, and description; use `--week-start monday|sunday` for the week boundary.\n");
    out.push_str("- Create/update time-entry commands print only the changed resource ID.\n");
    out.push_str("- `today` and `yesterday` are valid date filters for `entry list` and resolve in the local process timezone.\n\n");
    out.push_str("## Configuration And Defaults\n\n");
    out.push_str("- API key resolution: `CLOCKIFY_API_KEY` then stored config.\n");
    out.push_str("- Workspace resolution for normal commands: `--workspace`, `CFD_WORKSPACE`, stored config.\n");
    out.push_str("- Project defaults apply where commands support stored project resolution.\n");
    out.push_str("- `cfd skill` becomes workspace- or project-specific only with explicit `--workspace` and optional `--project`.\n\n");
    out.push_str("## Troubleshooting\n\n");
    out.push_str("- `missing Clockify API key` means login/config/env credentials are absent.\n");
    out.push_str("- `missing workspace` means pass `--workspace`, set `CFD_WORKSPACE`, or store a workspace.\n");
    out.push_str("- If an entry mutation rounds to an invalid interval, retry with `--no-rounding` or adjust timestamps.\n");
    out.push_str("- If a workspace/project/task/tag ID is unknown, list the parent collection with `--format json` and use returned IDs.\n\n");
}

fn workspace_flag(workspace: Option<&SkillWorkspaceContext>) -> String {
    workspace
        .map(|workspace| format!(" --workspace {}", workspace.id))
        .unwrap_or_default()
}

fn project_id(project: Option<&SkillProjectContext>) -> &str {
    project
        .map(|project| project.id.as_str())
        .unwrap_or("<project-id>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn args(flags: &[(&str, &str)]) -> ParsedArgs {
        ParsedArgs {
            resource: Some("skill".into()),
            action: None,
            subaction: None,
            positional: vec![],
            flags: flags
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            output: Default::default(),
            workspace: flags
                .iter()
                .find(|(key, _)| *key == "workspace")
                .map(|(_, value)| (*value).to_string()),
            yes: false,
            no_rounding: false,
        }
    }

    fn workspace() -> SkillWorkspaceContext {
        SkillWorkspaceContext {
            id: "w1".into(),
            name: "Engineering Platform".into(),
        }
    }

    fn project() -> SkillProjectContext {
        SkillProjectContext {
            id: "p1".into(),
            name: "Platform".into(),
        }
    }

    #[test]
    fn missing_scope_defaults_to_standard() {
        assert_eq!(SkillScope::parse(None).unwrap(), SkillScope::Standard);
    }

    #[test]
    fn parses_supported_scopes() {
        assert_eq!(SkillScope::parse(Some("brief")).unwrap(), SkillScope::Brief);
        assert_eq!(
            SkillScope::parse(Some("standard")).unwrap(),
            SkillScope::Standard
        );
        assert_eq!(SkillScope::parse(Some("full")).unwrap(), SkillScope::Full);
    }

    #[test]
    fn rejects_invalid_scope() {
        let error = SkillScope::parse(Some("nope")).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid scope: nope. Expected one of: brief, standard, full"
        );
    }

    #[test]
    fn validates_supported_formats() {
        validate(&args(&[])).unwrap();
        validate(&args(&[("format", "text")])).unwrap();
        validate(&args(&[("format", "md")])).unwrap();
    }

    #[test]
    fn rejects_json_raw_and_unknown_formats() {
        for format in ["json", "raw", "xml"] {
            let error = validate(&args(&[("format", format)])).unwrap_err();
            assert_eq!(
                error.to_string(),
                "cfd skill only supports --format text or --format md"
            );
        }
    }

    #[test]
    fn output_has_frontmatter_with_time_tracking_boundary() {
        let text = render_skill(SkillScope::Brief, None, None);
        assert!(text.starts_with("---\nname: clockify\ndescription: >-\n"));
        assert!(text.contains("Clockify time tracking"));
        assert!(text.contains("work logs"));
        assert!(text.contains("not generic issue tracker work logs"));
    }

    #[test]
    fn workspace_suffix_is_sanitized() {
        let workspace = SkillWorkspaceContext {
            id: "w1".into(),
            name: "Engineering_Platform 42!".into(),
        };
        let text = render_skill(SkillScope::Brief, Some(&workspace), None);
        assert!(text.contains("name: clockify-engineering-platform-42\n"));
    }

    #[test]
    fn project_name_takes_precedence_for_skill_name() {
        let text = render_skill(SkillScope::Brief, Some(&workspace()), Some(&project()));
        assert!(text.contains("name: clockify-platform\n"));
        assert!(!text.contains("name: clockify-engineering-platform\n"));
    }

    #[test]
    fn workspace_context_only_appears_when_workspace_exists() {
        let generic = render_skill(SkillScope::Brief, None, None);
        assert!(!generic.contains("## Workspace Context"));

        let text = render_skill(SkillScope::Brief, Some(&workspace()), None);
        assert!(text.contains("## Workspace Context"));
        assert!(text.contains("- Name: Engineering Platform"));
        assert!(text.contains("- ID: w1"));
    }

    #[test]
    fn project_context_only_appears_when_project_exists() {
        let workspace = workspace();
        let generic = render_skill(SkillScope::Brief, Some(&workspace), None);
        assert!(!generic.contains("## Project Context"));

        let text = render_skill(SkillScope::Brief, Some(&workspace), Some(&project()));
        assert!(text.contains("## Project Context"));
        assert!(text.contains("- Name: Platform"));
        assert!(text.contains("- ID: p1"));
    }

    #[test]
    fn regeneration_command_uses_effective_scope() {
        let text = render_skill(SkillScope::Standard, None, None);
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
        assert!(text.contains("cfd --version"));
        assert!(text.contains("cfd skill --scope standard > SKILL.md"));
    }

    #[test]
    fn workspace_regeneration_command_uses_resolved_id() {
        let text = render_skill(SkillScope::Full, Some(&workspace()), None);
        assert!(text.contains("cfd skill --workspace w1 --scope full > SKILL.md"));
    }

    #[test]
    fn project_regeneration_command_uses_resolved_ids() {
        let text = render_skill(SkillScope::Full, Some(&workspace()), Some(&project()));
        assert!(text.contains("cfd skill --workspace w1 --project p1 --scope full > SKILL.md"));
    }

    #[test]
    fn project_examples_use_resolved_project_id() {
        let text = render_skill(SkillScope::Standard, Some(&workspace()), Some(&project()));
        assert!(text.contains("cfd task list --workspace w1 --project p1 --format json"));
        assert!(text.contains(
            "cfd entry list --workspace w1 --start today --end today --format json --sort asc"
        ));
        assert!(text.contains("`--sort asc|desc`"));
        assert!(text.contains("cfd entry add --workspace w1 --start <iso> --duration <duration> --project p1 --description \"<work>\""));
        assert!(text.contains("cfd entry update --workspace w1 <entry-id> --end <iso>"));
        assert!(text.contains("cfd entry update --workspace w1 <entry-id> --duration <duration>"));
        assert!(
            text.contains("cfd entry update --workspace w1 <entry-id> --description \"<work>\"")
        );
        assert!(text
            .contains("cfd entry text list --workspace w1 --project p1 --columns text,lastUsed"));
        assert!(text
            .contains("cfd timer start \"ABC-1: Implement feature\" --workspace w1 --project p1"));
    }

    #[test]
    fn scopes_increase_in_size_and_detail() {
        let brief = render_skill(SkillScope::Brief, None, None);
        let standard = render_skill(SkillScope::Standard, None, None);
        let full = render_skill(SkillScope::Full, None, None);

        assert!(brief.len() < standard.len());
        assert!(standard.len() < full.len());
        assert!(!brief.contains("## Common Recipes"));
        assert!(standard.contains("## Common Recipes"));
        assert!(full.contains("## Command Reference"));
        assert!(full.contains("## Troubleshooting"));
    }

    #[test]
    fn rejects_missing_workspace_value() {
        let mut flags = HashMap::new();
        flags.insert("workspace".to_string(), "true".to_string());
        let parsed = ParsedArgs {
            resource: Some("skill".into()),
            action: None,
            subaction: None,
            positional: vec![],
            flags,
            output: Default::default(),
            workspace: Some("true".into()),
            yes: false,
            no_rounding: false,
        };

        let error = validate(&parsed).unwrap_err();
        assert_eq!(
            error.to_string(),
            "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]"
        );
    }

    #[test]
    fn rejects_project_without_workspace() {
        let error = validate(&args(&[("project", "p1")])).unwrap_err();
        assert_eq!(
            error.to_string(),
            "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]"
        );
    }

    #[test]
    fn rejects_missing_project_value() {
        let error = validate(&args(&[("workspace", "w1"), ("project", "true")])).unwrap_err();
        assert_eq!(
            error.to_string(),
            "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]"
        );
    }
}
