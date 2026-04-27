# 14 Timer Resume

## Goal

Verify `timer resume` for human terminal workflows:

- interactive recent-entry selection
- direct `-1` and `-2` selection
- default-yes direct confirmation
- `-y` confirmation skip
- copied project, task, tag, and description fields

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Optional confirmed task in that project
- Optional confirmed tag in that workspace
- Permission to create, stop, and delete time entries
- No timer is running before the journey starts, or permission to stop the existing timer first

## Steps

1. Run `cfd workspace list`.
2. Ask the user to confirm which workspace to use.
3. Run `cfd project list --workspace <confirmed-workspace-id>`.
4. Ask the user to confirm which project to use.
5. Optional: run `cfd task list --workspace <confirmed-workspace-id> --project <confirmed-project-id>` and ask whether to use a task.
6. Optional: run `cfd tag list --workspace <confirmed-workspace-id>` and ask whether to use a tag.
7. Choose two future timestamps later than the current time and later than any existing workspace entries for today, for example `<future-older-start-iso>` and `<future-newest-start-iso>` 20 minutes apart. This keeps the original test entries above any resumed timers that are created during the journey.
8. Create an older closed entry:
   `RESUME_OLDER_ID=$(cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <future-older-start-iso> --duration 10m --description "[CFD-TEST] resume older" --no-rounding -y)`
9. Create a newer closed entry, adding confirmed task/tag flags if selected:
   `RESUME_NEWEST_ID=$(cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <future-newest-start-iso> --duration 10m --description "[CFD-TEST] resume newest" --no-rounding -y)`
10. Verify ordering before direct resume:
    `cfd entry list --workspace <confirmed-workspace-id> --start today --end <after-future-newest-iso> --columns id,start,description,task`
11. Confirm the first two rows are `$RESUME_NEWEST_ID` followed by `$RESUME_OLDER_ID`.
12. Run `cfd timer resume -1 --workspace <confirmed-workspace-id>`.
13. Confirm the selected-entry display contains `[CFD-TEST] resume newest`.
14. Press Enter at `Resume this entry? [Y/n]:`.
15. Confirm stdout is only the new timer entry ID.
16. Run `cfd timer current --workspace <confirmed-workspace-id>`.
17. Confirm current output contains copied project and description, and copied task/tag if those were used.
18. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
19. Run `cfd timer resume -2 --workspace <confirmed-workspace-id> -y`.
20. Confirm it starts from `[CFD-TEST] resume older` without showing `Resume this entry?`.
21. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
22. Run interactive selection with an explicit start after the future test entries to avoid overlap prompts from earlier resumed timers:
    `cfd timer resume --workspace <confirmed-workspace-id> --start <after-future-newest-iso> --no-rounding`
23. Confirm the recent-entry list is newest-first, press Enter at `Select entry [0]:`, and verify it starts from `[CFD-TEST] resume newest`.
24. Stop the resumed timer with an explicit end after the explicit start:
    `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
25. If fewer than nine resumable entries are shown in this workspace, confirm missing selector failure:
    `cfd timer resume -9 --workspace <confirmed-workspace-id> -y`
26. Delete all entries created by this journey:
    `cfd entry delete <entry-id> --workspace <confirmed-workspace-id> -y`

## Expected Results

- `timer resume -1` maps to the newest resumable entry.
- `timer resume -2` maps to the second-newest resumable entry.
- The `-1` and `-2` checks use future-dated seed entries so stopped resume timers created during the journey do not displace the seed entries in the recent-entry list.
- Direct selection displays the selected entry before starting.
- The direct confirmation prompt is `Resume this entry? [Y/n]:`.
- Pressing Enter at the direct confirmation prompt starts the timer.
- `-y` skips the direct confirmation prompt.
- Interactive `timer resume` lists recent entries and Enter selects index `0`.
- Successful resume prints only the new timer ID on stdout.
- Resumed timers copy project, task, tags, and description from the selected entry.
- Resumed timers use fresh start times and do not reuse the selected entry interval.
- `timer resume -9 -y` fails clearly when fewer than nine resumable entries exist.

## Cleanup

- Stop any timer started by this journey.
- Delete entries created by this journey:
  - `[CFD-TEST] resume older`
  - `[CFD-TEST] resume newest`
  - all resumed timer entries
