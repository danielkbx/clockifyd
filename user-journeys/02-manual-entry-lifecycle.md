# 02 Manual Entry Lifecycle

## Goal

Verify basic time-entry CRUD without relying on timer flows.

## Preconditions

- Confirmed workspace
- A valid project ID for entry creation

## Steps

1. Add an entry with explicit `--start` and `--duration`
2. Capture the returned entry ID
3. Get the entry by ID
4. Update the entry description
5. List entries in a window that includes the new entry
6. Delete the entry

## Expected Results

- Create returns only the new entry ID on stdout
- Get returns the created entry
- Update returns only the entry ID
- List includes the entry in the expected window
- Delete succeeds

## Cleanup

- Delete the created entry if it still exists
