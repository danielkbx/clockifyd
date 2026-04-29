# 14 Timer Resume

## Goal

Verify `timer resume` for human terminal workflows:

- interactive recent-entry selection
- direct `-1` and `-2` selection
- default-yes direct confirmation
- `-y` confirmation skip
- interactive candidate count with `-nX`
- interactive description filter
- interactive task-name filter when a task is available
- rejection of `-nX` and filters with direct selectors
- copied project, task, tag, and description fields

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Optional confirmed task in that project; task-name filter coverage requires a confirmed task
- Optional confirmed tag in that workspace
- Permission to create, stop, and delete time entries
- No timer is running before the journey starts, or permission to stop the existing timer first
- If no task is available, mark the task-name filter check as skipped and still run the description filter checks

## Steps

1. Run `cfd workspace list`.
2. Ask the user to confirm which workspace to use.
3. Run `cfd project list --workspace <confirmed-workspace-id>`.
4. Ask the user to confirm which project to use.
5. Optional: run `cfd task list --workspace <confirmed-workspace-id> --project <confirmed-project-id>` and ask whether to use a task.
6. Optional: run `cfd tag list --workspace <confirmed-workspace-id>` and ask whether to use a tag.
7. Choose three future timestamps later than the current time and later than any existing workspace entries for today, for example `<future-older-start-iso>`, `<future-filter-start-iso>`, and `<future-newest-start-iso>` 20 minutes apart. Also choose `<after-future-newest-iso>` after all three seed entries, plus distinct explicit resume intervals after that point: `<interactive-start-iso>`/`<after-interactive-start-iso>`, `<limit-start-iso>`/`<after-limit-start-iso>`, `<description-filter-start-iso>`/`<after-description-filter-start-iso>`, and, if task coverage is run, `<task-filter-start-iso>`/`<after-task-filter-start-iso>`. This keeps the original test entries above any resumed timers that are created during the journey and avoids overlap prompts between the journey's explicit resume checks.
8. Create an older closed entry:
   `RESUME_OLDER_ID=$(cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <future-older-start-iso> --duration 10m --description "[CFD-TEST] resume older" --no-rounding -y)`
9. Create a newer closed entry for description-filter coverage:
   `RESUME_FILTER_ID=$(cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <future-filter-start-iso> --duration 10m --description "[CFD-TEST] resume filter needle" --no-rounding -y)`
10. Create the newest closed entry, adding confirmed task/tag flags if selected:
   `RESUME_NEWEST_ID=$(cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --start <future-newest-start-iso> --duration 10m --description "[CFD-TEST] resume newest" --no-rounding -y)`
11. If a task was confirmed, verify `$RESUME_NEWEST_ID` was created with that task so the task-name filter can find it.
12. Verify ordering before direct resume:
    `cfd entry list --workspace <confirmed-workspace-id> --start today --end <after-future-newest-iso> --columns id,start,description,task`
13. Confirm the first three seeded entries are newest-first, with `$RESUME_NEWEST_ID` before `$RESUME_FILTER_ID` before `$RESUME_OLDER_ID`.
14. Run `cfd timer resume -1 --workspace <confirmed-workspace-id>`.
15. Confirm the selected-entry display contains `[CFD-TEST] resume newest`.
16. Press Enter at `Resume this entry? [Y/n]:`.
17. Confirm stdout is only the new timer entry ID.
18. Run `cfd timer current --workspace <confirmed-workspace-id>`.
19. Confirm current output contains copied project and description, and copied task/tag if those were used.
20. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
21. Run `cfd timer resume -2 --workspace <confirmed-workspace-id> -y`.
22. Confirm it starts from `[CFD-TEST] resume filter needle` without showing `Resume this entry?`.
23. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
24. Run interactive selection with an explicit start after the future test entries to avoid overlap prompts from earlier resumed timers:
    `cfd timer resume --workspace <confirmed-workspace-id> --start <interactive-start-iso> --no-rounding`
25. Confirm the recent-entry list is newest-first, press Enter at `Select entry [0]:`, and verify it starts from `[CFD-TEST] resume newest`.
26. Stop the resumed timer with an explicit end after the explicit start:
    `cfd timer stop --workspace <confirmed-workspace-id> --end <after-interactive-start-iso> --no-rounding -y`
27. Run interactive selection limited to one candidate:
    `cfd timer resume -n1 --workspace <confirmed-workspace-id> --start <limit-start-iso> --no-rounding`
28. Confirm only one candidate row is displayed, row `0` is `[CFD-TEST] resume newest`, press Enter at `Select entry [0]:`, and confirm stdout is only the new timer entry ID.
29. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --end <after-limit-start-iso> --no-rounding -y`
30. Run interactive selection filtered by description:
    `cfd timer resume "filter needle" --workspace <confirmed-workspace-id> --start <description-filter-start-iso> --no-rounding`
31. Confirm the candidate list contains `[CFD-TEST] resume filter needle`, does not contain `[CFD-TEST] resume newest`, press Enter at `Select entry [0]:`, and verify it starts from the filtered entry.
32. Stop the resumed timer:
    `cfd timer stop --workspace <confirmed-workspace-id> --end <after-description-filter-start-iso> --no-rounding -y`
33. If a task was confirmed, run interactive selection filtered by a distinctive fragment of the confirmed task name:
    `cfd timer resume "<confirmed-task-name-fragment>" --workspace <confirmed-workspace-id> --start <task-filter-start-iso> --no-rounding`
34. If a task was confirmed, confirm the candidate list includes the entry with the confirmed task, select it, and run `cfd timer current --workspace <confirmed-workspace-id>` to verify the task was copied. If no task was confirmed, mark this check as skipped.
35. If a task-filter timer was started, stop it:
    `cfd timer stop --workspace <confirmed-workspace-id> --end <after-task-filter-start-iso> --no-rounding -y`
36. Confirm direct selector plus filter fails without starting a timer:
    `cfd timer resume -1 "filter needle" --workspace <confirmed-workspace-id> -y`
37. Confirm the command exits non-zero and reports that filters are interactive-only or shows usage clearly.
38. Confirm direct selector plus `-nX` fails without starting a timer:
    `cfd timer resume -1 -n2 --workspace <confirmed-workspace-id> -y`
39. Confirm the command exits non-zero and reports that `-n` is interactive-only or cannot be combined with direct selectors.
40. Confirm invalid `-n` fails without starting a timer:
    `cfd timer resume -n0 --workspace <confirmed-workspace-id>`
41. Confirm the command exits non-zero and reports an invalid resume count.
42. If fewer than nine resumable entries are shown in this workspace, confirm missing selector failure:
    `cfd timer resume -9 --workspace <confirmed-workspace-id> -y`
43. Delete all entries created by this journey:
    `cfd entry delete <entry-id> --workspace <confirmed-workspace-id> -y`

## Expected Results

- `timer resume -1` maps to the newest resumable entry.
- `timer resume -2` maps to the second-newest resumable entry.
- The `-1` and `-2` checks use future-dated seed entries so stopped resume timers created during the journey do not displace the seed entries in the recent-entry list.
- Direct `-1` and `-2` behavior remains unchanged by interactive filters and `-nX`.
- Direct selection displays the selected entry before starting.
- The direct confirmation prompt is `Resume this entry? [Y/n]:`.
- Pressing Enter at the direct confirmation prompt starts the timer.
- `-y` skips the direct confirmation prompt.
- Interactive `timer resume` lists recent entries and Enter selects index `0`.
- Resume candidate rows display the local start date and time for every entry, including today.
- `timer resume -nX` changes only the interactive list size.
- `timer resume "text"` filters only the interactive list.
- Filters match description case-insensitively.
- Filters match task name when task metadata is available.
- `-nX` and filters cannot be combined with direct `-1` through `-9`.
- Successful resume prints only the new timer ID on stdout.
- Resumed timers copy project, task, tags, and description from the selected entry.
- Resumed timers use fresh start times and do not reuse the selected entry interval.
- `timer resume -9 -y` fails clearly when fewer than nine resumable entries exist.

## Cleanup

- Stop any timer started by this journey.
- Delete entries created by this journey:
  - `[CFD-TEST] resume older`
  - `[CFD-TEST] resume filter needle`
  - `[CFD-TEST] resume newest`
  - all resumed timer entries created by `-nX` and filter checks
  - all resumed timer entries
