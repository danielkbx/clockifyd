# 12 Today Summary

## Goal

Verify `cfd today` as the fast daily overview for a real Clockify workspace.

## Preconditions

- Confirmed workspace
- At least one time entry exists today, or permission to create and clean up a temporary entry
- Optional: a running timer may exist to verify in-progress display

## Steps

1. Run `cfd workspace list`
2. Ask the user to confirm which workspace to use
3. Run `cfd project list --workspace <confirmed-workspace-id>`
4. Ask the user whether to use an existing project for an optional temporary entry
5. Run `cfd today --workspace <confirmed-workspace-id>`
6. Run `cfd today --workspace <confirmed-workspace-id> --format json`
7. Run `cfd entry list --workspace <confirmed-workspace-id> --start today --end today --format json`
8. Compare the JSON output from steps 6 and 7 semantically
9. If there is no entry today and the user approved a temporary entry, create one:
   `cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <today-iso> --duration 15m --description "cfd today journey"`
10. Run `cfd today --workspace <confirmed-workspace-id>` again
11. If the user approves testing a running entry and no timer is running, run:
    `cfd timer start --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "cfd today running journey"`
12. Run `cfd today --workspace <confirmed-workspace-id>` while the timer is running
13. Stop the timer if it was created by this journey:
    `cfd timer stop --workspace <confirmed-workspace-id> -y`
14. Capture returned entry IDs from any temporary entries
15. Delete temporary entries created by this journey

## Expected Results

- `cfd today` renders an ASCII table using only ASCII borders: `+`, `-`, and `|`
- Table headers appear in this exact order: `Project`, `Task`, `Description`, `Time`, `Duration`
- The final table row starts with `Total`
- Completed entries show local time ranges like `09:00-09:15`
- Running entries, when present, show `HH:MM-now`
- Running entries count toward the total
- JSON output is valid JSON
- JSON output matches `entry list --start today --end today --format json` semantically
- JSON output does not include a synthetic total row
- `--workspace <confirmed-workspace-id>` is honored consistently

## Cleanup

- Stop any timer started by this journey
- Delete all temporary entries created by this journey
- Do not delete entries that existed before the journey
