pub fn render_help(
    resource: Option<&str>,
    action: Option<&str>,
    subaction: Option<&str>,
) -> String {
    match (resource, action, subaction) {
        (None | Some("help"), _, _) => global_help(),
        (Some("login"), _, _) => {
            "Usage: cfd login\n\nPrompt for the Clockify API key and optionally store a default workspace, default project, and default rounding.".into()
        }
        (Some("logout"), _, _) => "Usage: cfd logout\n\nRemove the stored config.".into(),
        (Some("skill"), _, _) => skill_help(),
        (Some("whoami"), _, _) => "Usage: cfd whoami\n\nShow the current user.".into(),
        (Some("workspace"), _, _) => workspace_help(),
        (Some("config"), _, _) => config_help(),
        (Some("alias"), _, _) => alias_help(),
        (Some("project"), _, _) => project_help(),
        (Some("client"), _, _) => client_help(),
        (Some("tag"), _, _) => tag_help(),
        (Some("task"), _, _) => task_help(),
        (Some("entry"), Some("text"), _) => entry_text_help(),
        (Some("entry"), _, _) => entry_help(),
        (Some("today"), _, _) => today_help(),
        (Some("timer"), _, _) => timer_help(),
        (Some("completion"), _, _) => completion_help(),
        (Some(other), _, _) => {
            format!("Unknown command: {other}\nRun `cfd help` for a list of commands.")
        }
    }
}

const HELP_WIDTH: usize = 36;

fn help_group(out: &mut String, title: &str, items: &[(&str, &str)]) {
    out.push_str(title);
    out.push_str(":\n");
    help_items(out, items);
    out.push('\n');
}

fn help_items(out: &mut String, items: &[(&str, &str)]) {
    for (command, description) in items {
        out.push_str(&format!("  {command:<HELP_WIDTH$}  {description}\n"));
    }
}

fn global_help() -> String {
    let mut out = String::from("cfd - Clockify CLI\n\n");
    out.push_str("Usage: cfd <command> [options]\n\n");
    out.push_str("Commands:\n\n");

    help_group(
        &mut out,
        "Core",
        &[
            ("login", "Interactive login"),
            ("logout", "Remove stored config"),
            ("whoami", "Show current user"),
            ("completion <bash|zsh|fish>", "Generate shell completions"),
            ("--version", "Show version"),
        ],
    );
    help_group(
        &mut out,
        "Agent Skills",
        &[("skill", "Print latest SKILL.md guidance for AI agents")],
    );
    help_group(
        &mut out,
        "Workspaces And Defaults",
        &[
            ("workspace list", "List workspaces"),
            ("workspace get <id>", "Get workspace details"),
            ("config", "Show stored config"),
            ("config interactive", "Interactively update stored defaults"),
            ("config set workspace <id>", "Store default workspace"),
            ("config get workspace", "Show stored workspace"),
            ("config unset workspace", "Remove stored workspace"),
            ("config set project <id>", "Store default project"),
            ("config get project", "Show stored project"),
            ("config unset project", "Remove stored project"),
            (
                "config set rounding <off|1m|5m|10m|15m>",
                "Store default rounding",
            ),
            ("config get rounding", "Show stored rounding"),
            ("config unset rounding", "Remove stored rounding"),
        ],
    );
    help_group(
        &mut out,
        "Metadata",
        &[
            ("project list", "List projects"),
            ("project get <id>", "Get project details"),
            ("client list", "List clients"),
            ("client get <id>", "Get client details"),
            ("tag list", "List tags"),
            ("tag get <id>", "Get tag details"),
            ("task list --project <id>", "List tasks"),
            ("task get <project-id> <task-id>", "Get task details"),
            ("task create --project <id> --name <text>", "Create task"),
        ],
    );
    help_group(
        &mut out,
        "Time Entries",
        &[
            ("entry list", "List time entries"),
            ("entry get <id>", "Get time entry"),
            ("entry text list", "List known entry texts"),
            ("entry add", "Create time entry"),
            ("entry update <id>", "Update time entry"),
            ("entry delete <id>", "Delete time entry"),
            ("today", "Show today's time entries"),
        ],
    );
    help_group(
        &mut out,
        "Timer",
        &[
            ("timer current", "Show running timer"),
            ("timer start", "Start timer"),
            ("timer stop", "Stop timer"),
            ("timer resume", "Start timer from recent entry"),
            ("alias create <name>", "Create or update timer alias"),
            ("alias list", "List configured aliases"),
            ("alias delete <name>", "Delete alias"),
            ("<alias> start", "Start timer through alias"),
        ],
    );

    out.push_str("Global flags:\n");
    help_items(
        &mut out,
        &[
            ("--format text|json|raw", "Output format; default: text"),
            ("--no-meta", "Suppress metadata where supported"),
            ("--workspace <id>", "Override configured workspace"),
            (
                "--no-rounding",
                "Disable configured rounding for one command",
            ),
            ("-y", "Skip confirmation prompts"),
        ],
    );
    out.push_str("\nAI agents can run `cfd skill` to get current cfd usage instructions.\n");
    out.push_str("Use `cfd skill --workspace <workspace-id> [--project <project-id>]` for workspace/project-specific examples.\n");
    out.push_str("\nRun `cfd help <command>` or `cfd <command> help` for command-specific help.");
    out
}

fn skill_help() -> String {
    "Usage:
  cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]

Generate the latest SKILL.md content for AI agents using cfd.

Agents can run this command themselves to fetch current cfd usage
instructions instead of relying on a stale checked-in skill file.
Redirect stdout to SKILL.md when a persistent skill file is wanted.

Options:
  --scope brief|standard|full   Detail level for the generated skill; default: standard
  --workspace <workspace-id>    Resolve workspace and include workspace-specific context/examples
  --project <project-id>        Resolve project in the workspace and include project-specific context/examples

Formats:
  --format text                 Print Markdown; default
  --format md                   Print Markdown

Examples:
  cfd skill
  cfd skill --scope brief
  cfd skill --workspace <workspace-id>
  cfd skill --workspace <workspace-id> --project <project-id> --scope full > SKILL.md"
        .into()
}

fn workspace_help() -> String {
    "Usage:
  cfd workspace list [--format json] [--no-meta] [--columns <list>]
  cfd workspace get <id> [--format json] [--no-meta]

Options:
  --format json       Print JSON output
  --no-meta           Suppress metadata in text output
  --columns <list>    Print selected tab-separated columns for list output

Available columns:
  id    Workspace ID
  name  Workspace name

Default text columns:
  id,name

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Example:
  cfd workspace list --columns id,name"
        .into()
}

fn config_help() -> String {
    "Usage:
  cfd config
  cfd config interactive
  cfd config set workspace <id>
  cfd config get workspace
  cfd config unset workspace
  cfd config set project <id>
  cfd config get project
  cfd config unset project
  cfd config set rounding <off|1m|5m|10m|15m>
  cfd config get rounding
  cfd config unset rounding

Show the full stored config, or manage stored CLI settings.

Keys:
  workspace    Default workspace ID
  project      Default project ID
  rounding     Default rounding mode: off, 1m, 5m, 10m, or 15m

Examples:
  cfd config
  cfd config set workspace <id>
  cfd config set rounding 15m"
        .into()
}

fn alias_help() -> String {
    "Usage:
  cfd alias create <alias> [--project <project-id>] [--task <task-id|none>] [--description <text|none>]
  cfd alias list [--format text|json|raw] [--no-meta]
  cfd alias delete <alias> [-y]
  cfd <alias> start [--start <iso>] [--no-rounding] [-y]

Aliases are local shortcuts for recurring timer starts.
They bind a project and can optionally bind a task and description.

Interactive create:
  When run in a terminal, missing project/task/description values are prompted.
  Defaults are shown by label, for example `Select Project [Project One]:`.

Examples:
  cfd alias create standup --project <project-id> --description \"Daily standup\"
  cfd standup start
  cfd alias delete standup -y"
        .into()
}

fn project_help() -> String {
    "Usage:
  cfd project list [--format json] [--no-meta] [--columns <list>]
  cfd project get <id> [--format json] [--no-meta]

Options:
  --format json       Print JSON output
  --no-meta           Suppress metadata in text output
  --columns <list>    Print selected tab-separated columns for list output

Available columns:
  id             Project ID
  name           Project name
  client         Client ID
  workspaceId    Workspace ID
  workspaceName  Workspace name

Default text columns:
  id,name

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Example:
  cfd project list --columns id,name,client,workspaceId,workspaceName"
        .into()
}

fn client_help() -> String {
    "Usage:
  cfd client list [--format json] [--no-meta] [--columns <list>]
  cfd client get <id> [--format json] [--no-meta]

Options:
  --format json       Print JSON output
  --no-meta           Suppress metadata in text output
  --columns <list>    Print selected tab-separated columns for list output

Available columns:
  id    Client ID
  name  Client name

Default text columns:
  id,name

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Example:
  cfd client list --columns id,name"
        .into()
}

fn tag_help() -> String {
    "Usage:
  cfd tag list [--format json] [--no-meta] [--columns <list>]
  cfd tag get <id> [--format json] [--no-meta]

Options:
  --format json       Print JSON output
  --no-meta           Suppress metadata in text output
  --columns <list>    Print selected tab-separated columns for list output

Available columns:
  id    Tag ID
  name  Tag name

Default text columns:
  id,name

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Example:
  cfd tag list --columns id,name"
        .into()
}

fn task_help() -> String {
    "Usage:
  cfd task list --project <id> [--format json] [--no-meta] [--columns <list>]
  cfd task get <project-id> <task-id> [--format json] [--no-meta]
  cfd task create --project <id> --name <text>

Options:
  --project <id>      Project ID
  --name <text>       Task name for create
  --format json       Print JSON output
  --no-meta           Suppress metadata in text output
  --columns <list>    Print selected tab-separated columns for list output

Available columns:
  id       Task ID
  name     Task name
  project  Project ID

Default text columns:
  id,name

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Example:
  cfd task list --project <id> --columns id,name,project

Create prints only the created task ID on stdout."
        .into()
}

fn entry_help() -> String {
    "Usage:
  cfd entry list --start <iso|today|yesterday> --end <iso|today|yesterday> [filters]
  cfd entry get <id> [--format json] [--no-meta] [--columns <list>]
  cfd entry text list [--project <id>] [--format json] [--no-meta] [--columns <list>]
  cfd entry add --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
  cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
  cfd entry delete <id> [-y]

Date keywords `today` and `yesterday` are resolved in the local process timezone.
`--columns` applies to text output for `entry list` and `entry get`.
It switches to a column view with one row per entry.

Filters:
  --project <id>      Project ID
  --task <id>         Task ID
  --tag <id>          Tag ID; may be repeated
  --text <value>      Description text filter

Fields:
  --project <id>      Project ID
  --task <id>         Task ID
  --tag <id>          Tag ID; may be repeated
  --description <text>
                      Entry description

Available columns:
  id           Entry ID
  start        Start timestamp
  end          End timestamp or `-` for running entries
  duration     ISO 8601 duration from Clockify, if present
  description  Entry description
  projectId    Project ID
  projectName  Project name
  task         Task ID
  tags         Comma-separated tag IDs

Default text columns:
  id,start,end,duration,description,projectId,projectName,task,tags

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Examples:
  cfd entry list --start today --end today --columns start,end,description
  cfd entry add --start 2026-04-27T09:00:00Z --duration 1h --project <id>
  cfd entry update <id> --start 2026-04-27T09:00:00Z --end 2026-04-27T10:00:00Z"
        .into()
}

fn entry_text_help() -> String {
    "Usage:
  cfd entry text list [--project <id>] [--format json] [--no-meta] [--columns <list>]

List previously used entry descriptions for one project.
Project is resolved from `--project` or stored config.

Available columns:
  text       Entry description
  lastUsed   Most recent usage timestamp
  count      Number of uses

Default text columns:
  text,lastUsed,count

Constraints:
  `--columns` requires a comma-separated list
  `--columns` cannot be combined with `--format`

Examples:
  cfd entry text list --project <id> --columns text,lastUsed"
        .into()
}

fn today_help() -> String {
    "Usage:
  cfd today [--format text|json|raw] [--workspace <id>] [--no-meta]

Show today's time entries as an ASCII table with a total row.

Text columns:
  Project, Task, Description, Time, Duration

Formats:
  --format text       ASCII table; default
  --format json       Raw time entry JSON array
  --format raw        Alias for JSON

Notes:
  Today is resolved in the local process timezone.
  Running entries are shown as HH:MM-now and count toward the total.
  Task displays the Clockify task ID.
  Use `entry list --start today --end today --columns ...` for tab-separated columns."
        .into()
}

fn timer_help() -> String {
    "Usage:
  cfd timer current [--format json] [--no-meta]
  cfd timer start [description] [--start <iso>] [fields...] [--no-rounding]
  cfd timer stop [--end <iso>] [--no-rounding] [-y]
  cfd timer resume [-1|-2|-3|-4|-5|-6|-7|-8|-9] [--start <iso>] [--no-rounding] [-y]"
        .to_string()
        + "

Fields:
  --project <id>       Project ID
  --task <id>          Task ID
  --tag <id>           Tag ID; may be repeated

Notes:
  Mutating timer commands apply configured rounding unless --no-rounding is set.
  timer start accepts the description as one optional positional argument.
  timer start uses the current time unless --start is set.
  timer resume copies project, task, tags, and description from a recent entry.
  timer resume without -1..-9 lists recent entries and prompts for a selection.
  timer resume -1 uses the newest entry, -2 the second newest, and so on.
  Direct resume prompts with default yes unless -y is set.
  timer stop and resume start paths check overlaps and ask for confirmation unless -y is set."
}

fn completion_help() -> String {
    "Usage:
  cfd completion <bash|zsh|fish>

Generate shell completions for Bash, Zsh, or Fish.
The generated script is written to stdout and does not require login.

Examples:
  cfd completion bash > ~/.local/share/bash-completion/completions/cfd
  cfd completion zsh > ~/.zfunc/_cfd
  cfd completion fish > ~/.config/fish/completions/cfd.fish"
        .into()
}

#[cfg(test)]
mod tests {
    use super::render_help;

    fn assert_columns_help(help: &str, example: &str) {
        assert!(help.contains("Available columns:"));
        assert!(help.contains("Default text columns:"));
        assert!(help.contains("cannot be combined with `--format`"));
        assert!(help.contains(example));
    }

    #[test]
    fn renders_entry_help() {
        let help = render_help(Some("entry"), None, None);
        assert!(help.contains("cfd entry list"));
        assert!(help.contains("cfd entry get <id> [--format json] [--no-meta] [--columns <list>]"));
        assert!(help.contains(
            "cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]"
        ));
        assert!(help.contains("today|yesterday"));
        assert!(help.contains("id,start,end,duration,description,projectId,projectName,task,tags"));
        assert!(help.contains("one row per entry"));
        assert_columns_help(&help, "--columns start,end,description");
    }

    #[test]
    fn renders_entry_text_help() {
        let help = render_help(Some("entry"), Some("text"), Some("list"));
        assert!(help.contains("cfd entry text list"));
        assert!(help.contains("[--project <id>]"));
        assert!(help.contains("stored config"));
        assert_columns_help(&help, "--columns text,lastUsed");
    }

    #[test]
    fn renders_today_help() {
        let help = render_help(Some("today"), None, None);

        assert!(help.contains("cfd today [--format text|json|raw]"));
        assert!(help.contains("Project, Task, Description, Time, Duration"));
        assert!(help.contains("Raw time entry JSON array"));
        assert!(help.contains("HH:MM-now"));
        assert!(help.contains("entry list --start today --end today --columns"));
    }

    #[test]
    fn renders_global_help_with_version_and_full_format_list() {
        let help = render_help(None, None, None);
        assert!(help.contains("Core:"));
        assert!(help.contains("Agent Skills:"));
        assert!(help.contains("Workspaces And Defaults:"));
        assert!(help.contains("Metadata:"));
        assert!(help.contains("Time Entries:"));
        assert!(help.contains("Timer:"));
        assert!(help.contains("Global flags:"));
        assert!(help.contains("today"));
        assert!(help.contains("--version"));
        assert!(help.contains("--format text|json|raw"));
        assert!(help.contains("timer stop"));
        assert!(help.contains("completion <bash|zsh|fish>"));
        assert!(help.contains("Generate shell completions"));
        assert!(help.contains("AI agents can run `cfd skill`"));
        assert!(help.contains("Run `cfd help <command>` or `cfd <command> help`"));
        assert!(!help.contains(
            "config set workspace <id>\n                            Store default workspace"
        ));
    }

    #[test]
    fn renders_completion_help() {
        let help = render_help(Some("completion"), None, None);

        assert!(help.contains("cfd completion <bash|zsh|fish>"));
        assert!(help.contains("Bash"));
        assert!(help.contains("Zsh"));
        assert!(help.contains("Fish"));
        assert!(help.contains("stdout"));
        assert!(help.contains("does not require login"));
    }

    #[test]
    fn renders_workspace_help_with_columns() {
        let help = render_help(Some("workspace"), None, None);
        assert!(help.contains("cfd workspace list"));
        assert_columns_help(&help, "--columns id,name");
    }

    #[test]
    fn renders_project_help_with_columns() {
        let help = render_help(Some("project"), None, None);
        assert!(help.contains("cfd project list"));
        assert_columns_help(&help, "--columns id,name,client");
    }

    #[test]
    fn renders_client_help_with_columns() {
        let help = render_help(Some("client"), None, None);
        assert!(help.contains("cfd client list"));
        assert_columns_help(&help, "--columns id,name");
    }

    #[test]
    fn renders_tag_help_with_columns() {
        let help = render_help(Some("tag"), None, None);
        assert!(help.contains("cfd tag list"));
        assert_columns_help(&help, "--columns id,name");
    }

    #[test]
    fn renders_task_help_with_columns() {
        let help = render_help(Some("task"), None, None);
        assert!(help.contains("cfd task list"));
        assert_columns_help(&help, "--columns id,name,project");
    }
}
