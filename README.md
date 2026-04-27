# cfd - Clockify CLI

`cfd` is a command-line client for Clockify. It works with workspaces, projects, clients, tags, tasks, time entries, running timers, stored defaults, and configurable rounding.

The default output is compact plain text. Use JSON when you want scriptable output, or `--columns` when you want tab-separated rows.

`cfd` can also generate current `SKILL.md` guidance for AI agents with `cfd skill`. Agents can run that command themselves to fetch up-to-date Clockify time tracking instructions, including workspace/project-specific examples with `cfd skill --workspace <workspace-id> --project <project-id>`.

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

Agents can also run `cfd skill` themselves to fetch up-to-date Clockify time tracking guidance instead of relying on stale checked-in instructions. Add `--workspace <workspace-id>` when the agent should receive workspace-specific examples, and add `--project <project-id>` when project-scoped examples should use a concrete project:

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
cfd entry list --start <iso|today|yesterday> --end <iso|today|yesterday> [--project <id>] [--task <id>] [--tag <id>...] [--text <value>] [--columns <list>]
cfd entry get <id> [--columns <list>]
cfd entry add --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry delete <id> [-y]

cfd today
```

Entry fields:

```bash
--project <id>
--task <id>
--tag <id>
--description <text>
```

`today` and `yesterday` use the local process timezone. Create and update commands print only the entry ID. Delete prompts unless `-y` is passed.

### Today Summary

```bash
cfd today
cfd today --format json
```

`cfd today` shows today's entries as an ASCII table with a total row. The text columns are `Project`, `Task`, `Description`, `Time`, and `Duration`. Running entries are displayed as `HH:MM-now` and count toward the total.

`--format json` and `--format raw` return the raw time-entry JSON array, matching `cfd entry list --start today --end today --format json`. Use `entry list --start today --end today --columns <list>` when you need tab-separated columns.

### Entry Text Reuse

```bash
cfd entry text list [--project <id>] [--columns <list>]
```

`entry text list` lists previously used descriptions for one project. The project comes from `--project` or the stored project default. Descriptions are deduplicated and sorted by most recent use.

```bash
cfd entry text list --columns text,lastUsed
```

### Timer

```bash
cfd timer current
cfd timer start [description] [--project <project-id>] [--task <task-id>] [--no-rounding]
cfd timer stop [--end <iso>] [--no-rounding] [-y]
```

`timer start` accepts the description as one optional positional argument. Use quotes for descriptions with spaces. `timer stop` uses the current time unless you pass an explicit `--end`.

### Aliases

Aliases are local shortcuts for recurring timer starts. They bind a project and can optionally bind a task and description.

```bash
cfd alias create <alias> [--project <project-id>] [--task <task-id|none>] [--description <text|none>]
cfd alias list
cfd alias delete <alias> [-y]
cfd <alias> start
```

`alias create` runs interactively in a terminal when values are missing. Defaults are displayed by label, for example `Select Project [Project One]:`. Use `--task none` or `--description none` to clear those optional fields when updating an alias.

```bash
cfd alias create standup --project <project-id> --description "Daily standup"
cfd standup start
```

### Agent Skills

```bash
cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]
```

`cfd skill` prints current `SKILL.md` guidance for AI agents working with Clockify time tracking through `cfd`. Without `--workspace`, it works without login. With `--workspace`, it resolves the workspace and includes workspace-specific context and examples. With `--workspace` plus `--project`, it also resolves the project and uses that project ID in project-scoped examples.

`cfd skill` supports `--format text` and `--format md`. Both print Markdown. `--format json` and `--format raw` are not supported for this command.

## Working With Output

Global output flags:

| Flag | Description |
|---|---|
| `--format text` | Plain text, default |
| `--format json` | JSON output for scripts |
| `--format raw` | Alias for `--format json` |
| `--no-meta` | Hide metadata fields where supported |
| `--columns <list>` | Print selected fields as tab-separated rows where supported |
| `--workspace <id>` | Override configured workspace |
| `--no-rounding` | Disable configured rounding for one command |
| `-y` | Skip confirmation prompts |

Text output is line-based:

```text
key: value
key: value
```

Lists separate items with a blank line. `--columns` produces no header row and prints one tab-separated row per item. `--columns` and `--format` are mutually exclusive.

Create and update commands that return one changed resource print only its ID on stdout, which makes them easy to use in scripts:

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
cfd entry list --start today --end today --columns start,end,duration,description
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
```

With project and task:

```bash
cfd timer start "ABC-1: Implement feature" --project <project-id> --task <task-id>
```

### Add A Manual Entry

```bash
cfd entry add --start 2026-04-26T09:00:00Z --duration 1h30m --project <project-id> --description "ABC-1: Implement feature"
```

### Update An Entry

```bash
cfd entry update <entry-id> --start 2026-04-26T09:00:00Z --duration 2h --description "ABC-1: Implement feature"
```

### List Today's Entries

```bash
cfd today
cfd entry list --start today --end today --columns start,end,duration,description
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
cfd entry add --start <iso> --duration 20m --no-rounding
cfd timer stop --no-rounding
```

Rounding applies to `entry add`, `entry update`, `timer start`, and `timer stop`.

When a mutating command would create overlapping entries for the current user, `cfd` warns on stderr and asks for confirmation. Use `-y` to continue without the prompt. If rounding causes `end <= start`, retry with `--no-rounding`.

## ID Formats

Use the `id` values printed by `cfd` as input to later commands.

| Type | Input |
|---|---|
| Workspace | Clockify workspace ID returned by `workspace list` |
| Project | Clockify project ID returned by `project list` |
| Client | Clockify client ID returned by `client list` |
| Tag | Clockify tag ID returned by `tag list` |
| Task | Clockify task ID plus project ID |
| Entry | Clockify time entry ID returned by `entry list`, `entry get`, `entry add`, `timer start`, or `timer stop` |

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
