# 03 Timer Lifecycle

## Goal

Verify timer start, current, and stop.

## Preconditions

- Confirmed workspace
- Optional project/task IDs if the timer uses metadata

## Steps

1. Start a timer
2. Run `cfd timer current`
3. Stop the timer
4. Capture the resulting entry ID
5. Get that entry by ID
6. Delete the created entry if appropriate for the workspace

## Expected Results

- Start succeeds
- Current returns one in-progress timer
- Stop returns the final entry ID
- Get shows a closed interval

## Cleanup

- Delete the created entry if it still exists
