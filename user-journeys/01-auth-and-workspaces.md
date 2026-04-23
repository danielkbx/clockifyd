# 01 Auth and Workspaces

## Goal

Verify authentication and workspace discovery.

## Preconditions

- API key available
- User has at least one workspace

## Steps

1. Run `cfd login`
2. Run `cfd whoami`
3. Run `cfd workspace list`
4. Run `cfd workspace get <confirmed-workspace-id>`
5. Run `cfd config set workspace <confirmed-workspace-id>`
6. Run `cfd config get workspace`

## Expected Results

- Login prompts for API key and optional default-workspace selection, then stores credentials successfully
- `whoami` resolves the current user
- Workspace list contains the confirmed workspace
- Workspace get returns the selected workspace
- Config stores and returns the workspace value

## Cleanup

- None
