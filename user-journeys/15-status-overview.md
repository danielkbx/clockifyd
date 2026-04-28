# 15 Status Overview

## Goal

Verify `cfd status` as the fast current-state overview for a real Clockify workspace.

## Preconditions

- Confirmed workspace
- At least one project available, or permission to create entries against an existing project
- Permission to create and clean up temporary time entries if the workspace has no suitable data
- Optional: permission to start and stop a temporary timer

## Steps

1. Run `cfd workspace list`
2. Ask the user to confirm which workspace to use
3. Run `cfd project list --workspace <confirmed-workspace-id>`
4. Ask the user to confirm which project to use for any temporary entries
5. Run `cfd status --workspace <confirmed-workspace-id>`
6. Run `cfd status --workspace <confirmed-workspace-id> --format json`
7. Run `cfd status --workspace <confirmed-workspace-id> --week-start monday`
8. Run `cfd status --workspace <confirmed-workspace-id> --week-start sunday`
9. If the output has no today entries and the user approved temporary entries, create two entries with the same project, task state, and description:
   `cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <today-iso-1> --duration 10m --description "cfd status journey"`
   `cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <today-iso-2> --duration 20m --description "cfd status journey"`
10. Run `cfd status --workspace <confirmed-workspace-id>` again
11. If the user approves testing a running timer and no timer is running, run:
    `cfd timer start "cfd status running journey" --workspace <confirmed-workspace-id> --project <confirmed-project-id>`
12. Run `cfd status --workspace <confirmed-workspace-id>` while the timer is running
13. Run `cfd status --workspace <confirmed-workspace-id> --format json` while the timer is running
14. Stop the timer if it was created by this journey:
    `cfd timer stop --workspace <confirmed-workspace-id> -y`
15. Capture returned entry IDs from temporary entries and timer stop
16. Delete temporary entries created by this journey

## Expected Results

- Text output has `Timer`, `Today`, and `Week` sections
- Timer section says `running: yes` when a timer is active and `running: no` otherwise
- Running timer details render as an ASCII table with project, task, description, and duration
- Timer, Today, and Week tables use matching column widths when a timer is running
- Today and Week sections render ASCII tables with `Project`, `Task`, `Description`, and `Duration` columns
- Today and Week sections group entries by project, task, and description
- Two temporary entries with the same project/task/description appear as one grouped row with the summed duration
- Missing task or description displays as `none`
- Project names display when they can be resolved
- Running entries count toward the Today and Week totals
- `--week-start monday` and `--week-start sunday` both succeed
- JSON output is valid JSON with `timer`, `today`, and `week` objects
- JSON output includes grouped summaries and duration totals rather than raw Clockify time-entry arrays
- `--workspace <confirmed-workspace-id>` is honored consistently

## Cleanup

- Stop any timer started by this journey
- Delete all temporary entries created by this journey
- Do not delete entries that existed before the journey
