# 11 Workspace Agent Skill Generation

## Goal

Verify that workspace-specific generated skill guidance resolves and embeds the confirmed workspace correctly.

## Preconditions

- Confirmed workspace
- Valid API key
- Agent has followed `user-journeys/PROCESS.md` workspace selection rules

## Steps

1. Run `cfd workspace list`
2. Ask the user to confirm which workspace to use
3. Run `cfd workspace get <confirmed-workspace-id>`
4. Run `cfd skill --workspace <confirmed-workspace-id> --scope brief`
5. Review the output as Markdown, not exact text
6. Run `cfd skill --workspace <confirmed-workspace-id> --scope standard`
7. Run `cfd skill --workspace <confirmed-workspace-id> --scope full`
8. Confirm the regeneration command preserves the resolved workspace ID and effective scope
9. Optionally redirect to a temporary file: `cfd skill --workspace <confirmed-workspace-id> --scope standard > /tmp/cfd-skill.md`, then inspect the file

## Expected Results

- The generated skill includes a `Workspace Context` section
- The confirmed workspace ID appears in workspace context
- The confirmed workspace name appears in workspace context when returned by `workspace get`
- Workspace-specific examples include `--workspace <confirmed-workspace-id>` where workspace context affects commands
- The regeneration command includes `--workspace <confirmed-workspace-id>`
- Keeping-current instructions remain present and understandable
- Scope behavior matches the generic journey: brief < standard < full in detail
- The skill remains about Clockify time tracking, not generic work logs
- No time entries, tasks, projects, clients, or tags are created or mutated

## Cleanup

- Remove any temporary generated `SKILL.md` file
- Unset temporary env vars
