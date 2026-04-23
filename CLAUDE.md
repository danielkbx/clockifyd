# clockifyd — Clockify CLI

A CLI tool for Clockify time tracking. Designed for both human and AI-agent use, with compact output to minimize context window usage.

## Project Status

Current scope is implemented. The repo now contains the production CLI, subprocess CLI tests, release workflows, Homebrew formula, and user journeys for real-workspace verification.

## Architecture

```text
src/
  main.rs         <- entry point, command routing
  args.rs         <- argument parsing
  client.rs       <- HTTP transport trait + Clockify API client
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
user-journeys/    <- end-to-end test scripts for AI agents
.agents/          <- project context files
```

Core logic (`client.rs`, `config.rs`, `types.rs`, `error.rs`) must have no CLI dependencies. Command handlers in `commands/` own formatting, prompting, and stdout/stderr behavior.

## Tech Stack

- Language: Rust
- HTTP: `ureq` 3
- JSON: `serde` + `serde_json`
- Build: `cargo build --release`
- Distribution: standalone binaries via GitHub Releases
- Auth: Clockify API key via `X-Api-Key`
- Config: `~/.config/cfd/config.json` (XDG)
- Repository: `https://github.com/danielkbx/clockifyd`

## Implemented Commands

```text
cfd help / cfd help <command> / cfd <command> help
cfd login / logout / whoami

cfd workspace list [--columns <list>]
cfd workspace get <id>

cfd config set workspace <id>
cfd config get workspace
cfd config unset workspace
cfd config set project <id>
cfd config get project
cfd config unset project
cfd config set rounding <off|1m|5m|10m|15m>
cfd config get rounding
cfd config unset rounding

cfd project list [--columns <list>]
cfd project get <id>
cfd client list [--columns <list>]
cfd client get <id>
cfd tag list [--columns <list>]
cfd tag get <id>
cfd task list --project <id> [--columns <list>]
cfd task get <project-id> <task-id>
cfd task create --project <id> --name "ABC-1: Implement something nice"

cfd entry list --start <iso|today|yesterday> --end <iso|today|yesterday> [--project <id>] [--task <id>] [--tag <id>...] [--text <value>] [--columns <list>]
cfd entry get <id>
cfd entry text list [--project <id>] [--columns <list>]
cfd entry add --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry update <id> --start <iso> (--end <iso> | --duration <d>) [fields...] [--no-rounding]
cfd entry delete <id> [-y]

cfd timer current
cfd timer start [fields...] [--no-rounding]
cfd timer stop [--end <iso>] [--no-rounding] [-y]
```

## Output Flags

| Flag | Description |
|---|---|
| `--format json` | JSON output |
| `--format text` | Plain text, no Markdown |
| `--no-meta` | Suppress metadata |
| `--columns <list>` | Compact tab-separated text output for list commands |
| `--workspace <id>` | Override configured workspace |
| `--no-rounding` | Disable configured rounding for this command |
| `-y` | Skip confirmation prompts |

Create and update commands should output only the created or updated resource ID on stdout.

Notes:

- Text output is line-based (`key: value`) by default, with blank lines between list items.
- `--format raw` remains accepted as an alias for `--format json`.
- `workspace list`, `project list`, `client list`, `tag list`, `task list`, `entry list`, and `entry text list` support `--columns <list>`.
- `entry get` also supports `--columns <list>`.
- `entry list` and `entry get` support `duration`, `projectId`, and `projectName` column names.
- `project list` supports `workspaceId` and `workspaceName` column names.
- `--columns` requires an explicit comma-separated list.
- `--columns` and `--format` are mutually exclusive.

## Configuration

```json
~/.config/cfd/config.json
{
  "apiKey": "clockify-api-key",
  "workspace": "64a687e29ae1f428e7ebe303",
  "project": "64a687e29ae1f428e7ebe399",
  "rounding": "15m"
}
```

Credential and settings resolution order:

- Workspace: CLI flag -> `CFD_WORKSPACE` -> config file -> error
- Rounding: `--no-rounding` -> `CFD_ROUNDING` -> config file -> off
- API key: `CLOCKIFY_API_KEY` -> config file -> error

`entry text list` resolves project from `--project` or stored config. `today` and `yesterday` are supported in entry-list date filters and resolve in the local process timezone.

## Core Principles

### Think Before Coding
Don't assume. Don't hide confusion. Surface tradeoffs. State assumptions explicitly. Ask clarifying questions before implementing.

### Simplicity First
Minimum code that solves the problem. Nothing speculative. No unrequested features, no single-use abstractions, no impossible error handling.

### Surgical Changes
Touch only what you must. Clean up only your own mess. Match existing style. Every changed line must trace to the user's request.

### Goal-Driven Execution
Define success criteria before coding. Write tests before fixes. Verify refactored code still passes. Loop until verified.

## Agent Files

Read these files at the start of any non-trivial task:

| File | Contents |
|---|---|
| `.agents/architect.md` | Directory structure, module boundary, API mapping |
| `.agents/reviewer.md` | Code review standards and report format |
| `.agents/tester.md` | Test types, conventions, user journeys |
| `.agents/memory.md` | API quirks and project decisions not obvious from code |
| `.agents/process.md` | Tooling commands, commit rules, env vars |
