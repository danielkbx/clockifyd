# 11 Workspace Agent Skill Generation

## Goal

Verify that workspace- and project-specific generated skill guidance resolves and embeds the confirmed workspace and project correctly.

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Valid API key
- Agent has followed `user-journeys/PROCESS.md` workspace selection rules

## Steps

1. Run `cfd workspace list`
2. Ask the user to confirm which workspace to use
3. Run `cfd workspace get <confirmed-workspace-id>`
4. Run `cfd project list --workspace <confirmed-workspace-id>`
5. Ask the user to confirm which project to use
6. Run `cfd skill --workspace <confirmed-workspace-id> --scope brief`
7. Review the output as Markdown, not exact text
8. Run `cfd skill --workspace <confirmed-workspace-id> --scope standard`
9. Run `cfd skill --workspace <confirmed-workspace-id> --scope full`
10. Run `cfd skill --workspace <confirmed-workspace-id> --project <confirmed-project-id> --scope brief`
11. Run `cfd skill --workspace <confirmed-workspace-id> --project <confirmed-project-id> --scope standard`
12. Run `cfd skill --workspace <confirmed-workspace-id> --project <confirmed-project-id> --scope full`
13. Confirm the regeneration commands preserve the resolved workspace ID, project ID when present, and effective scope
14. Optionally redirect to a temporary file: `cfd skill --workspace <confirmed-workspace-id> --project <confirmed-project-id> --scope standard > /tmp/cfd-skill.md`, then inspect the file

## Expected Results

- The generated skill includes a `Workspace Context` section
- The confirmed workspace ID appears in workspace context
- The confirmed workspace name appears in workspace context when returned by `workspace get`
- Workspace-specific examples include `--workspace <confirmed-workspace-id>` where workspace context affects commands
- The project-specific generated skill includes a `Project Context` section
- The confirmed project ID appears in project context
- The confirmed project name appears in project context when returned by the project API
- Project-specific examples include `--project <confirmed-project-id>` where project context affects commands
- The regeneration command includes `--workspace <confirmed-workspace-id>`
- The project-specific regeneration command includes `--workspace <confirmed-workspace-id> --project <confirmed-project-id>`
- Keeping-current instructions remain present and understandable
- Scope behavior matches the generic journey: brief < standard < full in detail
- The skill remains about Clockify time tracking, not generic work logs
- No time entries, tasks, projects, clients, or tags are created or mutated

## Cleanup

- Remove any temporary generated `SKILL.md` file
- Unset temporary env vars
