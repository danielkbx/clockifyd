# 03 Timer Lifecycle

## Goal

Verify timer start with positional description, current, and stop.

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Permission to start, stop, and delete a timer entry
- No timer is running before the journey starts, or permission to stop the existing timer first

## Steps

1. Run `cfd workspace list`.
2. Ask the user to confirm which workspace to use.
3. Run `cfd project list --workspace <confirmed-workspace-id>`.
4. Ask the user to confirm which project to use.
5. Start a timer with positional description:
   `TIMER_ENTRY_ID=$(cfd timer start "[CFD-TEST] timer positional description" --workspace <confirmed-workspace-id> --project <confirmed-project-id> --no-rounding -y)`
6. Run `cfd timer current --workspace <confirmed-workspace-id>`.
7. Confirm current output contains `[CFD-TEST] timer positional description`.
8. Stop the timer:
   `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
9. Capture the stopped entry ID from the stop output when possible.
10. Run `cfd entry get <created-entry-id> --workspace <confirmed-workspace-id>`.
11. Confirm the entry description is `[CFD-TEST] timer positional description`.
12. Confirm deprecated timer description flag fails:
    `cfd timer start --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] should fail"`
13. Confirm multi-token unquoted description fails:
    `cfd timer start Run extra --workspace <confirmed-workspace-id> --project <confirmed-project-id>`
14. Delete the created entry:
    `cfd entry delete <created-entry-id> --workspace <confirmed-workspace-id> -y`

## Expected Results

- Positional description starts a timer.
- Current returns one in-progress timer.
- Current output shows `[CFD-TEST] timer positional description`.
- Stop returns the final entry ID.
- Get shows a closed interval with description `[CFD-TEST] timer positional description`.
- `--description` for `timer start` exits non-zero and does not start a timer.
- Multiple positional description tokens exit non-zero and do not start a timer.

## Cleanup

- Stop any timer started by this journey.
- Delete the `[CFD-TEST] timer positional description` entry if it still exists.
