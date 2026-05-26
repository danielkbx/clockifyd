# cfd - Clockify CLI

[![CI](https://github.com/danielkbx/clockifyd/actions/workflows/ci.yml/badge.svg)](https://github.com/danielkbx/clockifyd/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/danielkbx/clockifyd)](https://github.com/danielkbx/clockifyd/releases/latest)

`cfd` is a command-line client for Clockify. It helps you browse Clockify data, track time, manage timers, and keep common workspace defaults close at hand.

The default output is compact plain text. Use JSON or selected columns when you want to script against the output.

## Installation

### Homebrew

```bash
brew tap danielkbx/tap
brew install cfd
```

The Homebrew formula installs Bash, Zsh, and Fish completion files to Homebrew's standard completion directories.

### Download

Download the latest archive for your platform from the [Releases](../../releases) page.

| Platform | Archive |
|---|---|
| Linux x86_64 | `cfd-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `cfd-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `cfd-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `cfd-aarch64-apple-darwin.tar.gz` |

```bash
tar xzf cfd-aarch64-apple-darwin.tar.gz
sudo mv cfd /usr/local/bin/
```

### Build From Source

Requires stable [Rust](https://rustup.rs/).

```bash
git clone https://github.com/danielkbx/clockifyd.git
cd clockifyd
cargo build --release
```

The binary is written to `target/release/cfd`.

## Shell Completions

`cfd` can generate completion scripts for `bash`, `zsh`, and `fish`.
Generated scripts are written to stdout and do not require login.

Homebrew installs generated completions automatically. The manual steps below are for downloaded binaries, source builds, or custom shell setups.

### Bash

Install and enable Bash completion support first. With Homebrew:

```bash
brew install bash-completion@2
```

Then add this to `~/.bashrc` or another Bash startup file:

```bash
[[ -r /opt/homebrew/etc/profile.d/bash_completion.sh ]] && source /opt/homebrew/etc/profile.d/bash_completion.sh
```

On Intel macOS/Homebrew installations, use `/usr/local/etc/profile.d/bash_completion.sh` instead.

```bash
mkdir -p ~/.local/share/bash-completion/completions
cfd completion bash > ~/.local/share/bash-completion/completions/cfd
```

Reload your shell, or source the generated file directly for the current session:

```bash
source ~/.local/share/bash-completion/completions/cfd
```

### Zsh

Zsh completion support is built in through `compinit`. Homebrew installs `_cfd` into its standard `site-functions` directory; most Homebrew Zsh setups only need:

```zsh
autoload -Uz compinit
compinit
```

For a manual install:

```bash
mkdir -p ~/.zfunc
cfd completion zsh > ~/.zfunc/_cfd
```

Add the completion directory to your `fpath` before `compinit` in `~/.zshrc`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit
compinit
```

Reload your shell after updating `~/.zshrc`.

### Fish

```fish
mkdir -p ~/.config/fish/completions
cfd completion fish > ~/.config/fish/completions/cfd.fish
```

Fish loads files from that directory automatically in new shells.

### Troubleshooting

Check whether your shell has loaded the completion:

```bash
complete -p cfd
```

```zsh
print $_comps[cfd]
```

```fish
complete --do-complete "cfd "
```

For Zsh, make sure custom completion directories are added to `fpath` before `compinit`. If an old completion is still used, clear the cache and reinitialize:

```zsh
rm -f ~/.zcompdump*
unfunction _cfd 2>/dev/null
autoload -Uz compinit
compinit -u
```

Some terminals provide their own completion UI. If `complete --do-complete "cfd "` shows `cfd` commands in Fish but pressing Tab shows only files, configure the terminal to use native shell completions.

## Getting Started

Log in with a Clockify API key:

```bash
cfd login
```

`cfd login` prompts for the API key, then can store a default workspace, default project, and default rounding mode.

Check the login:

```bash
cfd whoami
```

Explore Clockify:

```bash
cfd workspace list --columns id,name
cfd project list --columns id,name,client
cfd client list --columns id,name
cfd tag list --columns id,name
cfd help
```

Use command-specific help whenever you need exact syntax:

```bash
cfd help entry
cfd entry help
cfd help timer
```

### AI Agent Skill File

`cfd` can generate a current `SKILL.md` file for AI agents:

```bash
cfd skill > SKILL.md
```

Agents can run `cfd skill` to get current Clockify time tracking guidance. Add `--workspace <workspace-id>` or `--project <project-id>` when the examples should use real workspace context:

```bash
cfd skill --workspace <workspace-id> > SKILL.md
cfd skill --workspace <workspace-id> --project <project-id> --scope full
```

## Command Guide

### Authentication And Help

```bash
cfd login
cfd logout
cfd whoami
cfd help
cfd help <command>
cfd <command> help
cfd --version
cfd completion <bash|zsh|fish>
cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]
```

### Workspaces And Defaults

```bash
cfd workspace list [--columns <list>]
cfd workspace get <id>

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

cfd alias create <alias> [--project <project-id>] [--task <task-id|none>] [--description <text|none>]
cfd alias list
cfd alias delete <alias> [-y]
cfd <alias> start
cfd <alias> switch
```

`cfd config interactive` updates stored workspace, project, and rounding defaults without asking for the API key again. Defaults reduce repeated `--workspace` and `--project` flags.

### Projects, Clients, Tags, And Tasks

```bash
cfd project list [--columns <list>]
cfd project get <id>

cfd client list [--columns <list>]
cfd client get <id>

cfd tag list [--columns <list>]
cfd tag get <id>

cfd task list --project <id> [--columns <list>]
cfd task get <project-id> <task-id>
cfd task create --project <id> --name "ABC-1: Implement something nice"
```

Tasks are created explicitly. `task create` prints only the created task ID on stdout.

### Time Entries

```bash
cfd entry list --start <time|today|yesterday> --end <time|today|yesterday> [--project <id>] [--task <id>] [--tag <id>...] [--text <value>] [--columns <list>] [--sort asc|desc]
cfd entry get <id> [--columns <list>]
cfd entry add --start <time> (--end <time> | --duration <d>) [fields...] [--no-rounding]
cfd entry update <id> [--start <time>] [--end <time> | --duration <d>] [fields...] [--no-rounding]
cfd entry delete <id> [-y]

cfd today [--sort asc|desc]
cfd timeline
cfd status [--week-start monday|sunday]

cfd split entry <id> --at <time> [--gap <duration>] [--no-rounding] [-y]
cfd split timer --at <time> [--gap <duration>] [--no-rounding] [-y]

cfd switch current
cfd switch start [description] [--start <time>] [--project <project-id>] [--task <task-id>] [--tag <tag-id>] [--no-rounding] [-y]
cfd switch stop [--end <time>] [--no-rounding] [-y]
```

Entry fields:

```bash
--project <id>
--task <id>
--tag <id>
--description <text>
```

For `entry update`, you only need to pass the fields you want to change. Existing values are kept for anything you omit.

### Relative Times

`--start` and `--end` accept ISO timestamps and short relative times.

Examples:

```bash
now
-5m
+30m
now-2h
now+1h30m
```

For most commands, values such as `-15m` are relative to the current time. For `entry update`, bare relative values adjust the existing entry time instead: `--end +10m` extends the current end by 10 minutes.

Use `now-5m` when updating an entry and you mean five minutes before now instead of five minutes before the stored end.

`today` and `yesterday` use your local timezone. `entry list` loads paginated Clockify results automatically and sorts oldest first by default; pass `--sort desc` to show newest entries first. Create and update commands print only the entry ID. Delete prompts unless `-y` is passed.

### Today Summary

```bash
cfd today
cfd today --format json
cfd today --sort desc
```

`cfd today` shows today's entries in a table with a total row. Running entries count toward the total. Entries are oldest first by default; use `--sort desc` to show newest entries first.

Use `--format json` for scriptable output. Use `entry list --start today --end today --columns <list>` when you need tab-separated columns.

### Interactive Timeline

```bash
cfd timeline
```

`cfd timeline` opens an interactive ASCII view of your time entries, grouped by day. It loads paginated Clockify results for visible date ranges automatically. Use it when you want to inspect and adjust your tracked time visually in the terminal.

`cfd timeline` requires an interactive terminal and does not support `--format` or `--columns`. Use `cfd today` or `cfd entry list` for scriptable output.

### Status Overview

```bash
cfd status
cfd status --week-start sunday
cfd status --format json
```

`cfd status` shows the current timer, a summary for today, and a summary for the current week. If a temporary switch is active, it also shows which timer will be resumed.

The week starts on Monday by default. Use `--week-start sunday` for a Sunday-to-Sunday week. Running entries count toward the displayed totals. The today and week summaries load paginated Clockify results automatically.

Use `--format json` for scriptable status output. `--columns` is not supported by `status`.

### Split Entries And Timers

```bash
cfd split entry <entry-id> --at <time>
cfd split entry <entry-id> --at <time> --gap 15m
cfd split timer --at <time>
cfd split timer --at <time> --gap 5m --no-rounding
```

`split` divides one existing time record into two records. Use `split entry` for a finished entry and `split timer` for the current running timer.

Both forms keep the original project, task, tags, and description. The command shows the updated original entry and the new entry it created.

`--at <time>` accepts the same timestamp forms as timers and entries: ISO timestamps, `now`, and relative values such as `-15m`, `+30m`, or `now-2h`.

Use `--gap <duration>` when you want time between the two resulting records.

Configured rounding applies to split timestamps unless you pass `--no-rounding`. If you need an exact split point or exact gap, use `--no-rounding`.

If the split would overlap existing entries, `cfd` warns and asks for confirmation. Use `-y` to skip the prompt.

### Entry Text Reuse

```bash
cfd entry text list [--project <id>] [--columns <list>]
```

`entry text list` shows descriptions you have used before for a project, newest first.

```bash
cfd entry text list --columns text,lastUsed
```

### Timer

```bash
cfd timer current
cfd timer start [description] [--start <time>] [--project <project-id>] [--task <task-id>] [--tag <tag-id>] [--no-rounding]
cfd timer stop [--end <time>] [--no-rounding] [-y]
cfd timer resume [filter] [-n<count>] [--start <time>] [--no-rounding] [-y]
cfd timer resume [-1|-2|-3|-4|-5|-6|-7|-8|-9] [--start <time>] [--no-rounding] [-y]
cfd timer switch resume [filter] [-n<count>] [--start <time>] [--no-rounding] [-y]
cfd timer switch resume [-1|-2|-3|-4|-5|-6|-7|-8|-9] [--start <time>] [--no-rounding] [-y]
```

`timer start` accepts the description as one optional positional argument. Use quotes for descriptions with spaces. `timer stop` uses the current time unless you pass an explicit `--end`.
If a temporary switch is active, `timer stop` behaves like `switch stop` and returns to the original timer.

`timer resume` starts a new timer from a recent time entry. Without a numeric selector it shows recent entries and prompts for a selection. Use `-n20` to show more entries, pass quoted filter text such as `cfd timer resume "review"`, or use `-1` through `-9` to pick a recent entry directly. The new timer copies project, task, tags, and description, but uses a fresh start time.

`timer switch resume` uses the same recent-entry selection as `timer resume`, including interactive filtering and `-1` through `-9`, then temporarily switches to the selected entry instead of starting it as a normal timer.

### Temporary Switches

```bash
cfd switch current
cfd switch current --format json
cfd switch start [description] [--start <time>] [--project <project-id>] [--task <task-id>] [--tag <tag-id>] [--no-rounding] [-y]
cfd switch stop [--end <time>] [--no-rounding] [-y]
cfd <alias> switch
```

`switch` temporarily moves from the current running timer to another timer, then returns to the original timer. This is useful when you need to handle a short interruption without losing the timer you were on.

`switch current` shows the temporary timer and the timer that will be resumed.

Configured rounding applies unless `--no-rounding` is present. Nested switches are not supported.

### Aliases

Aliases are local shortcuts for recurring timer starts. They bind a project and can optionally bind a task and description.

```bash
cfd alias create <alias> [--project <project-id>] [--task <task-id|none>] [--description <text|none>]
cfd alias list
cfd alias delete <alias> [-y]
cfd <alias> start
cfd <alias> switch
```

`alias create` runs interactively in a terminal when values are missing. Defaults are displayed by label, for example `Select Project [Project One]:`. Use `--task none` or `--description none` to clear those optional fields when updating an alias.

```bash
cfd alias create standup --project <project-id> --description "Daily standup"
cfd standup start
cfd standup switch
```

### Agent Skills

```bash
cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]
```

`cfd skill` prints Markdown guidance for AI agents that use `cfd` for Clockify time tracking. It can include workspace and project examples when you pass those IDs.

`cfd skill` supports `--format text` and `--format md`. Both print Markdown. `--format json` and `--format raw` are not supported for this command.

## Working With Output

Global output flags:

| Flag | Description |
|---|---|
| `--format text` | Plain text, default |
| `--format json` | JSON output for scripts |
| `--format raw` | Same output as `--format json` |
| `--no-meta` | Hide extra metadata where supported |
| `--columns <list>` | Print selected fields as tab-separated rows where supported |
| `--sort asc|desc` | Sort entry timeline output by start time where supported |
| `--week-start monday|sunday` | Week boundary for `status` |
| `--workspace <id>` | Override configured workspace |
| `--no-rounding` | Disable configured rounding for one command |
| `-y` | Skip confirmation prompts |

Plain text output uses simple `key: value` lines:

```text
key: value
key: value
```

Lists separate items with a blank line. `--columns` prints one tab-separated row per item and cannot be combined with `--format`. `entry list` and `today` support `--sort asc|desc`; `asc` is the default and places the newest entry last. `status` does not support `--columns`.

Create and update commands print only the changed ID, which makes them easy to use in scripts:

```bash
ENTRY_ID=$(cfd entry add --start 2026-04-26T09:00:00Z --duration 30m --project <project-id> --description "Planning")
cfd entry get "$ENTRY_ID"
```

Delete and overlap confirmations can be skipped with `-y`.

## Columns Mode

Available columns:

| Command | Columns |
|---|---|
| `workspace list` | `id`, `name` |
| `project list` | `id`, `name`, `client`, `workspaceId`, `workspaceName` |
| `client list` | `id`, `name` |
| `tag list` | `id`, `name` |
| `task list` | `id`, `name`, `project` |
| `entry list`, `entry get` | `id`, `start`, `end`, `duration`, `description`, `projectId`, `projectName`, `task`, `tags` |
| `entry text list` | `text`, `lastUsed`, `count` |

Examples:

```bash
cfd workspace list --columns id,name
cfd project list --columns id,name,workspaceName
cfd task list --project <project-id> --columns id,name,project
cfd entry list --start today --end today --columns start,end,duration,description --sort asc
cfd entry text list --columns text,lastUsed,count
```

## Common Workflows

### Configure Defaults

```bash
cfd login
cfd config interactive
cfd config get workspace
cfd config get project
cfd config get rounding
```

### Start And Stop A Timer

```bash
cfd timer start "ABC-1: Implement feature"
cfd timer current
cfd timer stop
cfd timer start "ABC-1: Implement feature" --start -10m
cfd timer stop --end now
```

With project and task:

```bash
cfd timer start "ABC-1: Implement feature" --project <project-id> --task <task-id>
```

### Add A Manual Entry

```bash
cfd entry add --start 2026-04-26T09:00:00Z --duration 1h30m --project <project-id> --description "ABC-1: Implement feature"
cfd entry add --start -45m --duration 45m --project <project-id> --description "ABC-1: Implement feature"
```

### Update An Entry

```bash
cfd entry update <entry-id> --end 2026-04-26T11:00:00Z
cfd entry update <entry-id> --end -5m
cfd entry update <entry-id> --end now-5m
cfd entry update <entry-id> --duration 2h
cfd entry update <entry-id> --description "ABC-1: Implement feature"
```

### List Today's Entries

```bash
cfd today
cfd entry list --start today --end today --columns start,end,duration,description --sort asc
cfd entry list --start today --end today --sort desc
cfd entry list --start -2h --end now --columns start,end,duration,description
```

### Check Current Status

```bash
cfd status
cfd status --week-start sunday
cfd status --format json
```

### Reuse A Prior Description

```bash
cfd entry text list --columns text,lastUsed
cfd entry add --start 2026-04-26T10:00:00Z --duration 45m --description "ABC-1: Implement feature"
```

### Create A Task

```bash
TASK_ID=$(cfd task create --project <project-id> --name "ABC-1: Implement feature")
cfd task get <project-id> "$TASK_ID"
```

## Rounding And Overlaps

Supported rounding modes are `off`, `1m`, `5m`, `10m`, and `15m`.

```bash
cfd config set rounding 15m
cfd config get rounding
cfd config unset rounding
```

Disable rounding for one command:

```bash
cfd entry add --start <time> --duration 20m --no-rounding
cfd timer stop --no-rounding
```

Rounding applies to commands that create or change time records, including entries, timers, switches, and splits. Relative times are interpreted first, then rounded.

When a command would create overlapping entries, `cfd` warns and asks for confirmation. Use `-y` to continue without the prompt. If rounding makes an entry too short, retry with `--no-rounding`.

## ID Formats

Use the `id` values printed by `cfd` as input to later commands.

| Type | Input |
|---|---|
| Workspace | Clockify workspace ID returned by `workspace list` |
| Project | Clockify project ID returned by `project list` |
| Client | Clockify client ID returned by `client list` |
| Tag | Clockify tag ID returned by `tag list` |
| Task | Clockify task ID plus project ID |
| Entry | Clockify time entry ID returned by `entry list`, `entry get`, `entry add`, `timer start`, `timer stop`, `timer resume`, or `split` |

`task get` requires both project ID and task ID.

## Configuration

Stored config file:

```text
~/.config/cfd/config.json
```

Example:

```json
{
  "apiKey": "clockify-api-key",
  "workspace": "64a687e29ae1f428e7ebe303",
  "project": "64a687e29ae1f428e7ebe399",
  "rounding": "15m",
  "aliases": {
    "standup": {
      "project": "64a687e29ae1f428e7ebe399",
      "task": "64a687e29ae1f428e7ebe400",
      "description": "Daily standup"
    }
  }
}
```

Environment variables:

| Variable | Purpose |
|---|---|
| `CLOCKIFY_API_KEY` | Clockify API key |
| `CFD_WORKSPACE` | Default workspace override |
| `CFD_ROUNDING` | Default rounding override |
| `CFD_CONFIG` | Custom config file path |

Resolution order:

- API key: `CLOCKIFY_API_KEY` -> config file
- Workspace: `--workspace` -> `CFD_WORKSPACE` -> config file
- Rounding: `--no-rounding` -> `CFD_ROUNDING` -> config file -> `off`

Use `CFD_CONFIG` for multiple Clockify setups:

```bash
alias cfd-work='CFD_CONFIG=~/.config/cfd/work.json cfd'
alias cfd-oss='CFD_CONFIG=~/.config/cfd/oss.json cfd'

CFD_CONFIG=~/.config/cfd/work.json cfd login
CFD_CONFIG=~/.config/cfd/oss.json cfd login
```

`cfd config` prints the stored config with the API key masked. `cfd login` stores credentials and can select defaults interactively.

## License

GPL-3.0-only - see [LICENSE.md](LICENSE.md).
