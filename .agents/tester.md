# Testing

## Test Runner

Rust's built-in test framework. Run with `cargo test`.

## Test Types

| Type | Location | When to run |
|---|---|---|
| Unit | `src/*.rs` inline `#[cfg(test)]` | Always |
| CLI | `tests/` | Always |
| User Journeys | `user-journeys/` | Manually by AI agent |

## Unit Tests

Target areas:

- `config.rs` - read/write/clear config, env precedence
- `args.rs` - command parsing variants
- `format.rs` - output options, line-based text rendering, JSON rendering, metadata suppression
- `duration.rs` - duration parsing and invalid inputs
- `datetime.rs` - rounding (`1m`, `5m`, `10m`, `15m`), half-up behavior, boundary cases
- `client.rs` - API methods with `MockTransport`
- `commands/entry.rs` - overlap detection, `--no-rounding`, self-exclusion for updates, `--columns` validation and rendering
- `commands/timer.rs` - rounding + overlap behavior
- `commands/task.rs` - create request shape and output contract
- `commands/login.rs` - interactive login flow and workspace selection
- `completion.rs` and `cli_spec.rs` - shell completion rendering and CLI drift checks

## CLI Integration Tests

Spawn `target/debug/cfd` as a subprocess. Assert on stdout, stderr, and exit code.

For each command, verify:

1. Happy path output for `--format json` and `--format text`
2. `--no-meta` suppresses metadata fields
3. Create/update commands print only the resource ID on stdout
4. Invalid input exits non-zero with a useful message on stderr
5. `--no-rounding` disables configured rounding for that invocation

Specific coverage:

1. `config set|get|unset workspace`
2. `config set|get|unset rounding`
3. Workspace resolution precedence is CLI flag -> `CFD_WORKSPACE` -> stored config
4. Rounding resolution precedence is `--no-rounding` -> `CFD_ROUNDING` -> stored config -> off
5. Rounded future timestamps are accepted
6. Overlap produces a warning and confirmation prompt
7. `-y` skips overlap confirmation
8. `end <= start` after rounding is rejected
9. `login` prompts for API key and default workspace selection
10. `config interactive` reuses an existing API key and updates workspace/project/rounding defaults
11. `workspace|project|client|tag|task|entry|entry text list --columns` produce one tab-separated row per item with no header
12. `entry get --columns` produces one tab-separated row with no header
13. bare `--columns` fails with a useful usage-style error
14. `--columns` and `--format` conflict clearly
15. `completion bash|zsh|fish` produces non-empty shell-specific output without requiring auth
16. `skill` produces current agent guidance without requiring auth unless `--workspace` is passed
17. `skill --workspace <id>` resolves and embeds workspace context

## Rust-specific Notes

- Config tests share env vars -> use a `Mutex` when mutating process env
- Use `tempfile::tempdir()` for filesystem isolation
- No external test framework needed

## User Journey Tests

Directory: `user-journeys/`

End-to-end tests that an AI agent runs against a real Clockify workspace. Every journey file describes a full flow with steps, expected results, and cleanup.

Important process rules:

- Before any journey, the agent must run `cfd workspace list`
- The agent must use the `workspace list` output to explicitly ask the user which workspace to use
- If a journey reads or writes project-scoped data, the agent should first run `cfd project list --workspace <confirmed-workspace-id>` when possible and use that output to explicitly ask the user which project to use
- The agent must wait for confirmation before proceeding
- The agent must wait for explicit confirmation of the chosen workspace and project before proceeding
- For config-isolation scenarios, the agent should use `CFD_CONFIG`

## Journey Set

| Journey | File | Covers |
|---|---|---|
| Auth & Workspaces | `01-auth-and-workspaces.md` | login, whoami, workspace list/get |
| Manual Entry Lifecycle | `02-manual-entry-lifecycle.md` | add/get/update/delete |
| Timer Lifecycle | `03-timer-lifecycle.md` | start/current/stop |
| Metadata Browse | `04-project-client-task-tag-browse.md` | projects, clients, tags, tasks |
| Task Creation | `05-task-create.md` | task create |
| Rounding & Overlaps | `06-rounding-and-overlaps.md` | rounding config, `--no-rounding`, overlap warning |
| Filters & Output | `07-filters-and-output.md` | list filters, formats, `--no-meta` |
| Defaults & Overrides | `08-workspace-defaults-and-overrides.md` | config/env/flag precedence |
| API Guard | `09-api-compat-guard.md` | request shapes and risky integrations |
| Agent Skill Generation | `10-agent-skill-generation.md` | generic skill output, scopes, update instructions |
| Workspace Agent Skill Generation | `11-workspace-agent-skill-generation.md` | workspace-specific skill output and semantic review |

## Conventions

- No shared test utilities unless used in 3+ test files
- Keep fixtures small
- Prefer testing command behavior over internal implementation details
