# 02 Manual Entry Lifecycle

## Goal

Verify time-entry CRUD without relying on timer flows, including update behavior that preserves omitted fields.

## Preconditions

- Confirmed workspace
- A valid project ID for entry creation
- Optional confirmed task ID in that project
- Optional confirmed tag ID in that workspace

## Steps

1. Add a temporary entry with explicit `--project`, `--start`, `--duration`, and `--description`
2. Include `--task` and/or `--tag` if confirmed
3. Capture the returned entry ID
4. Get the entry by ID and verify project, optional task/tag, description, start, and end
5. Update only the end:
   `cfd entry update <id> --end <new-end-iso> --no-rounding -y`
6. Get the entry by ID and verify:
   - start is unchanged
   - end changed
   - project is unchanged
   - task is unchanged if used
   - tag is unchanged if used
   - description is unchanged
7. Update only the duration:
   `cfd entry update <id> --duration 45m --no-rounding -y`
8. Get the entry by ID and verify:
   - start is unchanged
   - end equals the existing start plus 45 minutes
   - project, task, tag, and description are unchanged
9. Update only the description:
   `cfd entry update <id> --description "[CFD-TEST] updated manual lifecycle"`
10. Get the entry by ID and verify:
   - description changed
   - start and end are unchanged
   - project, task, and tag are unchanged
11. List entries in a window that includes the new entry
12. Delete the entry

## Expected Results

- Create returns only the new entry ID on stdout
- Get returns the created entry
- Update returns only the entry ID
- End-only updates preserve omitted fields from the existing entry
- Duration-only updates use the existing start time to calculate the new end
- Metadata-only updates preserve existing time interval and project/task/tag fields
- List includes the entry in the expected window
- Delete succeeds

## Cleanup

- Delete the created entry if it still exists
