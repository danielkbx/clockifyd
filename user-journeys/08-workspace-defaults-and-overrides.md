# 08 Workspace Defaults and Overrides

## Goal

Verify precedence rules for workspace and rounding.

## Preconditions

- Confirmed workspace
- Temporary config path available

## Steps

1. Set `CFD_CONFIG` to a temp config file
2. Store a workspace via `cfd config set workspace`
3. Verify `cfd config get workspace`
4. Override with `CFD_WORKSPACE`
5. Override again with `--workspace`
6. Store rounding via `cfd config set rounding 15m`
7. Override with `CFD_ROUNDING=10m`
8. Verify `--no-rounding` bypasses both

## Expected Results

- Workspace precedence: CLI flag -> env -> config
- Rounding precedence: `--no-rounding` -> env -> config -> off

## Cleanup

- Remove temp config file
- Unset temp env vars
