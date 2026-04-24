# clockifyd — Clockify CLI

CLI for Clockify time tracking. Single binary, no runtime dependencies, optimized for both humans and AI agents with compact default output.

## Status

Implemented and release-ready for the current planned scope:

- auth and config management
- interactive login with workspace selection
- workspace, project, client, tag, and task browse commands
- task creation
- entry list/get/add/update/delete
- `entry text list` for reusing prior descriptions
- timer current/start/stop
- configurable rounding, overlap warnings, line-based text output, JSON output, and `--columns` for all `list` commands

## Installation

### Homebrew

```bash
brew tap danielkbx/tap
brew install cfd
```

The Homebrew formula installs Bash, Zsh, and Fish completion files to Homebrew's standard completion directories.

### Download

Grab the latest binary for your platform from the [Releases](../../releases) page:

| Platform | Archive |
|---|---|
| Linux x86_64 | `cfd-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `cfd-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `cfd-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `cfd-aarch64-apple-darwin.tar.gz` |

### Build from Source

Requires stable Rust.

```bash
git clone https://github.com/danielkbx/clockifyd.git
cd clockifyd
cargo build --release
```

## Shell Completions

`cfd` can generate completion scripts for `bash`, `zsh`, and `fish`.
Generated scripts are written to stdout.

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

```bash
cfd login
cfd whoami
cfd config
cfd config interactive
cfd config set rounding 15m
cfd workspace list --columns id,name
cfd task create --project <project-id> --name "ABC-1: Implement something nice"
cfd entry text list --columns text,lastUsed
cfd entry list --start today --end today --columns start,end,description
cfd timer start --project <project-id> --description "ABC-1: implement something nice"
```

## Commands

### Core

```text
cfd help / cfd help <command> / cfd <command> help
cfd --version
cfd completion <bash|zsh|fish>
cfd login
cfd logout
cfd whoami
```

### Workspaces

```text
cfd workspace list [--columns <list>]
cfd workspace get <id>
```

### Config

```text
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
```

### Metadata

```text
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

### Time Entries

```text
cfd entry list --start <iso|today|yesterday> --end <iso|today|yesterday> [--project <id>] [--task <id>] [--tag <id>...] [--text <value>] [--columns <list>]
cfd entry get <id>
cfd entry text list [--project <id>] [--columns <list>]
cfd entry add --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry delete <id> [-y]
```

### Timer

```text
cfd timer current
cfd timer start [fields...] [--no-rounding]
cfd timer stop [--end <iso>] [--no-rounding] [-y]
```

## Output Flags

| Flag | Description |
|---|---|
| `--format json` | JSON output |
| `--format text` | Plain text (default) |
| `--no-meta` | Suppress metadata columns in text output |
| `--workspace <id>` | Override configured workspace |
| `--no-rounding` | Disable configured rounding for this invocation |
| `-y` | Skip overlap confirmation prompts |

Create and update commands print only the resource ID on stdout.

Notes:

- Text output is line-based (`key: value`) by default, with blank lines between list items.
- `--format raw` is still accepted as an alias for `--format json`.
- `workspace list`, `project list`, `client list`, `tag list`, `task list`, `entry list`, and `entry text list` support `--columns <list>` for a row-based column view.
- `entry get` also supports `--columns <list>`.
- `entry list` and `entry get` support `duration`, `projectId`, and `projectName` column names.
- `--columns` and `--format` are mutually exclusive.
- `--version` prints the CLI version and exits.

## Configuration

Config file: `~/.config/cfd/config.json`

```json
{
  "apiKey": "clockify-api-key",
  "workspace": "64a687e29ae1f428e7ebe303",
  "project": "64a687e29ae1f428e7ebe399",
  "rounding": "15m"
}
```

Resolution order:

- Workspace: CLI flag -> `CFD_WORKSPACE` -> config
- Rounding: `--no-rounding` -> `CFD_ROUNDING` -> config -> `off`
- API key: `CLOCKIFY_API_KEY` -> config

`cfd login` is interactive:

1. prompts for the Clockify API key
2. loads workspaces with that key
3. lets you choose a default workspace or `none`
4. shows only workspace names in the interactive selection
5. if a default workspace was selected, lets you choose a default project or `none`
6. shows only project names in the interactive selection
7. lets you choose default rounding or `none`

`cfd config interactive` runs the same workspace/project/rounding flow, but reuses the existing API key from env or config instead of prompting for it.

`cfd config` prints the full stored config and masks the API key, showing only the first 3 and last 3 characters.

## Description Reuse

Ticket references do not need a dedicated field. A common workflow is to store them in the entry description or task name:

```text
ABC-1: Implement something nice
```

`cfd entry text list` returns previously used descriptions for the current project, deduplicated and sorted by most recent use. That makes repeated ticket-based logging fast for both humans and agents.

## Columns Mode

All `list` commands support `--columns <list>` in text mode for a compact row-based view:

- `workspace list`
- `project list`
- `client list`
- `tag list`
- `task list`
- `entry list`
- `entry text list`

`entry get` also supports `--columns <list>`.

Available columns by command:

- `workspace list`: `id`, `name`
- `project list`: `id`, `name`, `client`, `workspaceId`, `workspaceName`
- `client list`: `id`, `name`
- `tag list`: `id`, `name`
- `task list`: `id`, `name`, `project`
- `entry list` and `entry get`: `id`, `start`, `end`, `duration`, `description`, `projectId`, `projectName`, `task`, `tags`
- `entry text list`: `text`, `lastUsed`, `count`

Rules:

- `--columns` requires an explicit comma-separated list
- output contains no header row
- each item is printed as exactly one tab-separated row
- `--columns` cannot be combined with `--format`

Examples:

```bash
cfd workspace list --columns id,name
cfd task list --project <project-id> --columns id,name,project
cfd entry text list --columns text,lastUsed
cfd entry list --start today --end today --columns start,end,description
```

Example output:

```text
w1	Engineering
t1	ABC-1: Implement something nice	p1
Focus work	2026-04-24T10:00:00Z	3
2026-04-23T09:00:00Z	2026-04-23T10:00:00Z	Focus
```

## Rounding and Overlap Confirmation

Supported rounding modes: `off`, `1m`, `5m`, `10m`, `15m`

Rounding applies to:

- `entry add`
- `entry update`
- `timer start`
- `timer stop`

`today` and `yesterday` are resolved in the local process timezone for list commands.

When a mutating command would create overlapping entries for the current user, `cfd` warns on `stderr` and asks for confirmation. Use `-y` to continue without the prompt. If rounding causes `end <= start`, retry with `--no-rounding`.

## License

GPL-3.0-only — see [LICENSE.md](LICENSE.md).
