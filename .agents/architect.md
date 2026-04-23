# Architecture

## Directory Structure

```text
src/
  main.rs           <- entry point, command routing, known-command validation
  args.rs           <- argument parsing (handwritten, no clap)
  client.rs         <- HttpTransport trait + ClockifyClient + UreqTransport
  config.rs         <- credential resolution + storage (XDG, mode 600)
  datetime.rs       <- timestamp parsing + configurable rounding
  duration.rs       <- parse_duration helpers
  error.rs          <- CfdError enum
  format.rs         <- OutputOptions, line-based text/JSON formatting, --no-meta
  help.rs           <- help text per command
  input.rs          <- input helpers (confirm, prompt, selection)
  types.rs          <- all data structures
  commands/
    mod.rs          <- module declarations
    login.rs        <- interactive login flow
    logout.rs       <- clear credentials
    whoami.rs       <- current user display
    config.rs       <- stored settings (workspace, project, rounding)
    workspace.rs    <- workspace list/get
    project.rs      <- project list/get
    client.rs       <- client list/get
    tag.rs          <- tag list/get
    task.rs         <- task list/get/create
    entry.rs        <- time-entry list/get/add/update/delete
    timer.rs        <- timer current/start/stop
```

## Module Boundary

Core modules (`client.rs`, `config.rs`, `types.rs`, `error.rs`) must have no CLI dependencies.

`commands/` own:

- stdout/stderr
- confirmation prompts
- interpretation of global flags
- final output formatting
- `entry list/get --columns` behavior and validation

## HttpTransport Trait

Planned shape:

```rust
pub trait HttpTransport {
    fn get(&self, url: &str, api_key: &str) -> Result<String, CfdError>;
    fn post(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn put(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn patch(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn delete(&self, url: &str, api_key: &str) -> Result<(), CfdError>;
}
```

Production transport: `UreqTransport`

Test transport: `MockTransport`

## Command Validation

Command names must be validated against a KNOWN map in `main.rs` before loading config. A typo must not produce misleading auth or workspace errors.

## Workspace Resolution

Resolution order:

1. `--workspace <id>`
2. `CFD_WORKSPACE`
3. stored config value
4. error

Interactive login may store the selected default workspace directly in config, but all later command resolution still follows the same precedence chain.

## Rounding Pipeline

For mutating time commands:

1. Parse user input into timestamps
2. Apply rounding unless `--no-rounding` is present
3. Validate resulting interval
4. Check overlaps
5. Ask for confirmation if overlaps exist and `-y` is not set
6. Build request payload
7. Send API request

Supported rounding modes:

- `off`
- `1m`
- `5m`
- `10m`
- `15m`

Rounding uses nearest-step semantics with half-up behavior on exact ties.

## Overlap Detection

Overlap warnings apply only to:

- `entry add`
- `entry update`
- `timer start`
- `timer stop`

Rules:

- same current user only
- same workspace only
- use final timestamps after rounding
- `entry update` excludes the updated entry itself from the check
- overlaps are warnings, not hard errors
- `-y` skips only the confirmation prompt

## Clockify API Mapping

Base URL:

```text
https://api.clockify.me/api/v1
```

Relevant endpoints for initial scope:

- `GET /v1/user`
- `GET /v1/workspaces`
- `GET /v1/workspaces/{workspaceId}/projects`
- `GET /v1/workspaces/{workspaceId}/clients`
- `GET /v1/workspaces/{workspaceId}/tags`
- `GET /v1/workspaces/{workspaceId}/projects/{projectId}/tasks`
- `POST /v1/workspaces/{workspaceId}/projects/{projectId}/tasks`
- `GET /v1/workspaces/{workspaceId}/user/{userId}/time-entries`
- `GET /v1/workspaces/{workspaceId}/time-entries/{id}`
- `POST /v1/workspaces/{workspaceId}/time-entries`
- `PUT /v1/workspaces/{workspaceId}/time-entries/{id}`
- `DELETE /v1/workspaces/{workspaceId}/time-entries/{id}`
- `GET /v1/workspaces/{workspaceId}/time-entries/status/in-progress`
- `PATCH /v1/workspaces/{workspaceId}/user/{userId}/time-entries`

## Config Module

`config.rs` owns:

- API key loading
- stored workspace
- stored project
- stored rounding
- XDG config path handling
- save with mode `600`
- clear config

## Output Contract

Default text output is line-based:

```text
field: value
field: value
```

List commands print blank lines between items.

JSON output uses `--format json`. `--format raw` is accepted as an alias for compatibility.

`entry list` and `entry get` have a second text mode via `--columns <list>`:

- no header row
- one tab-separated row per entry
- available columns: `id,start,end,description,project,task,tags`
- mutually exclusive with `--format`

## Adding a Command

1. Add API method on `ClockifyClient` in `client.rs`
2. Add handler in `commands/<resource>.rs`
3. Register in the KNOWN map and top-level router
4. Add unit and CLI tests
5. Update `README.md` and `CLAUDE.md`
