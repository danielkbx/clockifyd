# Project Memory

Discoveries and decisions not derivable from the code. Append new entries. Do not rewrite existing entries unless they are factually wrong.

---

## Clockify API: Authentication uses X-Api-Key
Date: 2026-04-23
All API requests in initial scope authenticate via the `X-Api-Key` header. The tool does not use OAuth.

## Clockify API: Workspaces come from GET /v1/workspaces
Date: 2026-04-23
`GET /v1/workspaces` is the canonical workspace listing endpoint for the current user and should back `workspace list`.

## Clockify API: User time entries are scoped by workspace and user
Date: 2026-04-23
Time-entry listing for overlap checks and normal listing is based on `GET /v1/workspaces/{workspaceId}/user/{userId}/time-entries`.

## Clockify API: Task creation is explicit
Date: 2026-04-23
`task create` maps to `POST /v1/workspaces/{workspaceId}/projects/{projectId}/tasks`. Task creation is intentionally separate from entry/timer commands to keep automation explicit.

## Clockify API: Time entry update requires start
Date: 2026-04-23
The documented update payload for `PUT /v1/workspaces/{workspaceId}/time-entries/{id}` includes required `start`. Build update logic accordingly.

## Clockify API: Pagination naming is endpoint-specific
Date: 2026-04-23
Clockify documentation is not fully uniform in query parameter naming. Use the documented parameter names for each endpoint exactly as shown.

## Product Rule: Rounding may create future timestamps
Date: 2026-04-23
If configured rounding moves a start time into the future, that is acceptable behavior and not an error.

## Product Rule: Overlap is warning, not blocker
Date: 2026-04-23
When entry add/update or timer start/stop would overlap existing entries, the CLI warns and asks for confirmation. `-y` skips the prompt.

## Product Rule: Supported rounding modes
Date: 2026-04-23
Supported modes are `off`, `1m`, `5m`, `10m`, and `15m`. `--no-rounding` disables configured rounding for a single command.

## Product Rule: Entry text reuse is project-scoped
Date: 2026-04-23
`entry text list` resolves its project from `--project` or stored config, trims descriptions, deduplicates by text, and sorts by most recent use descending.

## Product Rule: List date keywords use local timezone
Date: 2026-04-23
For `entry list`, the keywords `today` and `yesterday` are resolved against the local process timezone before conversion to UTC for the API request.

## Product Rule: Login is interactive
Date: 2026-04-23
`login` prompts for the Clockify API key, loads workspaces with that key, and lets the user choose a default workspace or `none`.

## Product Rule: Default text output is line-based
Date: 2026-04-23
Text output is no longer tabular by default. Objects render as `key: value` lines, and list commands separate items with a blank line.

## Product Rule: JSON is the official machine format
Date: 2026-04-23
`--format json` is the documented JSON mode across the CLI. `--format raw` remains supported as a compatibility alias.

## Product Rule: Entry columns mode is headerless
Date: 2026-04-23
`entry list|get --columns <list>` switches text output into a tab-separated row mode with no header row. The user-selected columns define the full row shape.

## Product Rule: Entry columns require explicit list and exclude format
Date: 2026-04-23
Bare `--columns` is invalid and should produce a usage-style error. `--columns` cannot be combined with `--format`.

## Product Rule: User-facing output uses camelCase
Date: 2026-04-23
Human-readable CLI output should use camelCase labels and column names for user-facing fields (for example `clientId`, `workspaceId`, `projectId`, `taskId`, `tagIds`, `lastUsed`). Internal Rust field names and JSON/API payloads may continue to use their existing naming where appropriate.

## Clockify API: tagIds may be null
Date: 2026-04-23
Time-entry responses may contain `tagIds: null`. Treat that as an empty tag list during deserialization.

## Reference
Date: 2026-04-23
Primary API reference: https://docs.clockify.me/
