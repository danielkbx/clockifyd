# 09 API Compat Guard

## Goal

Guard against drift in the Clockify API integration points that matter most for the CLI.

## Preconditions

- Confirmed workspace
- Valid API key

## Checks

1. `whoami` still works against the expected user endpoint
2. `workspace list` still maps to the documented workspace endpoint
3. `task create` still accepts the documented request shape
4. `entry list` still uses documented query parameter names
5. `entry update` still requires a valid `start`
6. `timer stop` still works through the documented stop endpoint

## Expected Results

- No undocumented request or response assumptions are required
- Any API drift is detected before broader feature work proceeds

## Cleanup

- Remove any temporary entities created during the checks
