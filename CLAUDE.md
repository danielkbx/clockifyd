# clockifyd - Clockify CLI

`cfd` is the released Clockify CLI for this repository. Current package version: `1.5.0`.

The public README is user documentation. This file and `.agents/*.md` are maintainer and agent context.

## Project Status

The CLI is released and the current feature surface is implemented. The repo contains:

- production Rust CLI
- subprocess CLI tests
- release workflows and Homebrew formula
- shell completion generation for Bash, Zsh, and Fish
- user journeys for real-workspace verification

## Architecture

```text
src/
  main.rs         <- entry point, command routing, known-command validation
  args.rs         <- argument parsing
  cli_spec.rs     <- canonical visible CLI model for completions and drift tests
  client.rs       <- HTTP transport trait + Clockify API client
  completion.rs   <- shell completion renderers
  config.rs       <- credential resolution + storage
  datetime.rs     <- timestamp parsing + rounding helpers
  duration.rs     <- duration parsing
  error.rs        <- error types
  format.rs       <- output formatting
  help.rs         <- help system
  input.rs        <- input helpers
  types.rs        <- all data structures
  commands/       <- command handlers
tests/            <- subprocess CLI coverage
user-journeys/    <- end-to-end test scripts for real Clockify workspaces
.agents/          <- maintainer and agent context files
```

Core logic (`client.rs`, `config.rs`, `types.rs`, `error.rs`) must have no CLI command dependencies. Command handlers in `commands/` own formatting, prompting, global flag behavior, and stdout/stderr behavior.

## Tech Stack

- Language: Rust
- HTTP: `ureq` 3
- JSON: `serde` + `serde_json`
- Build: `cargo build --release`
- Distribution: standalone binaries via GitHub Releases and Homebrew formula
- Auth: Clockify API key via `X-Api-Key`
- Config: `~/.config/cfd/config.json` (XDG)
- Repository: `https://github.com/danielkbx/clockifyd`

## Command Surface

```text
cfd help / cfd help <command> / cfd <command> help
cfd --version
cfd completion <bash|zsh|fish>
cfd skill [--scope brief|standard|full] [--workspace <id> [--project <id>]]
cfd login / logout / whoami

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

cfd project list [--columns <list>]
cfd project get <id>
cfd client list [--columns <list>]
cfd client get <id>
cfd tag list [--columns <list>]
cfd tag get <id>
cfd task list --project <id> [--columns <list>]
cfd task get <project-id> <task-id>
cfd task create --project <id> --name "ABC-1: Implement something nice"

cfd entry list --start <iso|today|yesterday> --end <iso|today|yesterday> [--project <id>] [--task <id>] [--tag <id>...] [--text <value>] [--columns <list>] [--sort asc|desc]
cfd entry get <id> [--columns <list>]
cfd entry text list [--project <id>] [--columns <list>]
cfd entry add --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry delete <id> [-y]

cfd today [--sort asc|desc]

cfd timer current
cfd timer start [description] [--project <id>] [--task <id>] [--tag <id>] [--no-rounding]
cfd timer stop [--end <iso>] [--no-rounding] [-y]
cfd timer resume [-1|-2|-3|-4|-5|-6|-7|-8|-9] [--start <iso>] [--no-rounding] [-y]
```

Timer aliases are local config entries under `aliases`. `alias create` is interactive in a TTY and must render defaults like ytd: `Select Project [Project One]:`, `Select Task [none]:`, and `Description [Existing description]:`. Use `--task none` and `--description none` to clear optional stored values.

Entry fields:

```text
--project <id>
--task <id>
--tag <id>
--description <text>
```

Timer start accepts `--project`, `--task`, and `--tag`; pass the description as one quoted positional argument.

## Output Contracts

| Flag | Description |
|---|---|
| `--format json` | JSON output |
| `--format text` | Plain text, default |
| `--format raw` | Compatibility alias for `--format json` |
| `--no-meta` | Suppress metadata where supported |
| `--columns <list>` | Compact tab-separated text output where supported |
| `--sort asc|desc` | Sort Entry timeline outputs by start time where supported |
| `--workspace <id>` | Override configured workspace |
| `--no-rounding` | Disable configured rounding for this command |
| `-y` | Skip confirmation prompts |

Create and update commands output only the created or updated resource ID on stdout.

Text output is line-based (`key: value`) by default, with blank lines between list items.

`entry list` and `today` sort by `timeInterval.start` ascending by default, so the newest entry appears last. Use `--sort desc` to show newest entries first. The selected order applies to default text, `--columns`, JSON, and raw output.

`cfd today` text output is an ASCII table with columns `Project`, `Task`, `Description`, `Time`, and `Duration`, plus a final `Total` row. Running entries display `HH:MM-now` and count toward the total. `cfd today --format json` and `--format raw` return the time-entry JSON array in the selected sort order.

`--columns` rules:

- supported by `workspace list`, `project list`, `client list`, `tag list`, `task list`, `entry list`, `entry text list`, and `entry get`
- requires an explicit comma-separated list
- emits no header row
- emits one tab-separated row per item
- cannot be combined with `--format`

Column names:

- `workspace list`: `id`, `name`
- `project list`: `id`, `name`, `client`, `workspaceId`, `workspaceName`
- `client list`: `id`, `name`
- `tag list`: `id`, `name`
- `task list`: `id`, `name`, `project`
- `entry list` and `entry get`: `id`, `start`, `end`, `duration`, `description`, `projectId`, `projectName`, `task`, `tags`
- `entry text list`: `text`, `lastUsed`, `count`

## Configuration

Config path:

```text
~/.config/cfd/config.json
```

Example:

```json
{
  "apiKey": "clockify-api-key",
  "workspace": "64a687e29ae1f428e7ebe303",
  "project": "64a687e29ae1f428e7ebe399",
  "rounding": "15m"
}
```

Credential and settings resolution order:

- API key: `CLOCKIFY_API_KEY` -> config file -> error
- Workspace: CLI flag -> `CFD_WORKSPACE` -> config file -> error
- Rounding: `--no-rounding` -> `CFD_ROUNDING` -> config file -> `off`

`entry text list` resolves project from `--project` or stored config. `today` and `yesterday` are supported in entry-list date filters and resolve in the local process timezone.

## Runtime Rules

- `login` prompts for the Clockify API key, validates credentials by loading workspaces, and can store default workspace/project/rounding.
- `config interactive` reuses the existing API key from env or config and updates workspace/project/rounding.
- Mutating time commands apply configured rounding unless `--no-rounding` is present.
- `timer resume` copies project/task/tags/description from a recent entry, uses a fresh start time, and supports `-1` through `-9` for direct recent-entry selection.
- Overlap warnings apply to `entry add`, `entry update`, `timer start`, `timer stop`, and `timer resume`.
- Overlap is warning plus confirmation, not a hard error.
- `-y` skips confirmation prompts but must not skip overlap detection.

## Core Principles

### Think Before Coding

Do not assume. Surface tradeoffs. State assumptions explicitly. Ask clarifying questions before implementing when product intent is unclear.

### Simplicity First

Use the minimum code that solves the problem. Avoid speculative features and single-use abstractions.

### Surgical Changes

Touch only what the task requires. Match existing style. Clean up only your own changes.

### Goal-Driven Execution

Define success criteria before coding. Write or update tests for behavior changes. Verify the relevant test set before finishing.

## Agent Files

Read these files at the start of any non-trivial task:

| File | Contents |
|---|---|
| `.agents/architect.md` | Directory structure, module boundary, API mapping |
| `.agents/reviewer.md` | Code review standards and report format |
| `.agents/tester.md` | Test types, conventions, user journeys |
| `.agents/memory.md` | API quirks and project decisions not obvious from code |
| `.agents/process.md` | Tooling commands, commit rules, env vars, documentation ownership |
