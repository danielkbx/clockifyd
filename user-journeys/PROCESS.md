# User Journey and Guard Test Process

## Preconditions

- `cfd` is built and available in `PATH` or as `./target/release/cfd`
- The agent is logged in and `cfd whoami` works

## Workflow

### 1. Workspace Selection

Before any journey is executed, the agent must:

1. Run `cfd workspace list`
2. Show the user the available workspaces
3. Explicitly ask which workspace should be used
4. If the journey reads or writes project-scoped data, run `cfd project list --workspace <confirmed-workspace-id>` when possible and show the user the available projects
5. Explicitly ask which project should be used for project-scoped commands
6. Wait for the user's confirmation of the selected workspace and, when applicable, project
7. Use the confirmed workspace ID consistently for the journey
8. Use only the confirmed project ID for project-scoped commands

For config-isolation scenarios, the agent should use `CFD_CONFIG` with a temporary config file.

### 2. Journey Execution

The files in this directory are either:

- user journeys
- technical guard checks

The agent:

1. Reads the selected file
2. Executes the steps in order
3. Verifies the expected result after every step
4. Performs cleanup even if a step fails

Skill-generation journeys are read-only and should not create Clockify resources. They are semantic reviews, not golden text tests: verify workspace inclusion, update instructions, time tracking trigger clarity, and scope-appropriate detail rather than exact wording.

### 2.1 Harness Rules

When automating journeys with a shell harness:

- Use a temporary copy of the selected config file:
  `CFD_CONFIG=$(mktemp /tmp/cfd-journey-config.XXXXXX.json)` followed by `cp <selected-config> "$CFD_CONFIG"`.
  This protects aliases, defaults, and `activeSwitch` state in the original config.
- Do not delete the temporary config in pre-run cleanup. Only remove it in the final exit trap.
- Capture created entry IDs from either compact ID output or expanded text output. Some commands, notably `timer stop`, can print expanded entry details when warnings such as overlap warnings are present. Prefer the first `id: <value>` line, falling back to a bare ID line:
  `awk '/^id: / {print $2; exit} /^[[:alnum:]]{20,}$/ {print; exit}'`.
- Quote fixed-string checks that begin with `-` by passing `--` to `rg`, for example:
  `rg -q --fixed-strings -- "-now"`.
- Use `[CFD-TEST]` in every temporary time-entry description, including switch journey Timer A/B descriptions. This makes cleanup reliable.
- Cleanup must search the full time window used by the harness, not only `today`, if any journey creates future-dated entries. Delete all `[CFD-TEST]` entries in that window before declaring cleanup complete.
- For `timer resume` direct selector checks, isolate ordering. Seed entries must be newer than unrelated workspace entries, and resumed copies created during the journey must not become candidates for later selector checks unless the journey explicitly expects that. Delete or time-position resumed copies so `-2` still refers to the intended second seed entry.
- For `timer switch resume`, ensure the intended resume target is newer than Timer A's closed switch-boundary entry. Otherwise `-1` may correctly select Timer A's just-closed entry instead of the intended seed entry.
- After all journeys, verify:
  - `cfd timer current --workspace <id>` reports no running timer
  - `cfd entry list --start <harness-window-start> --end <harness-window-end> --text "[CFD-TEST]" --format json` is empty
  - any non-prefixed temporary descriptions used by older journeys, such as `CFD journey`, are also absent
  - the temporary config has no aliases and no `activeSwitch`

### 3. Naming Convention

All test entities should use this prefix when possible:

- `[CFD-TEST]`

Examples:

- Task name: `[CFD-TEST] ABC-1: Implement something nice`
- Entry description: `[CFD-TEST] pair programming`

### 4. Cleanup Rules

| Entity | Cleanup |
|---|---|
| Time entry | delete it |
| Task | delete it if the scenario created it and the API supports cleanup in the workflow |
| Temp config file | `rm -f` |
| Temp env vars | `unset` |

### 5. Failure Handling

- If a step fails, still perform cleanup
- Document the failing step, command, and output
- Summarize pass/fail status to the user after cleanup

### 6. Recommended Order

1. `01-auth-and-workspaces.md`
2. `02-manual-entry-lifecycle.md`
3. `03-timer-lifecycle.md`
4. `04-project-client-task-tag-browse.md`
5. `05-task-create.md`
6. `06-rounding-and-overlaps.md`
7. `07-filters-and-output.md`
8. `08-workspace-defaults-and-overrides.md`
9. `09-api-compat-guard.md`
10. `10-agent-skill-generation.md`
11. `11-workspace-agent-skill-generation.md`
12. `12-today-summary.md`
13. `13-timer-aliases.md`
14. `14-timer-resume.md`
15. `15-status-overview.md`
16. `16-relative-datetime.md`
17. `17-temporary-switch.md`
18. `18-split.md`
