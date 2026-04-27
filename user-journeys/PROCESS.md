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
