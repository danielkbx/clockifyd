# Interactive Timeline

## Purpose

Verify that `cfd timeline` opens a responsive human-only TUI, fills the available terminal height with two-row day timelines, loads older visible days in week-sized background batches without blocking navigation, and supports inline entry actions from the cursor.

## Preconditions

- `cfd` is built locally.
- The tester has a real Clockify API key configured or available through `CLOCKIFY_API_KEY`.
- The tester can run commands in an interactive terminal, not a pipe or CI-only shell.
- The tester may create and delete disposable `[CFD-TEST]` time entries in the confirmed workspace.

## Setup

1. Run `cfd workspace list`.
2. Ask the user which workspace ID to use.
3. Wait for explicit workspace confirmation.
4. Run `cfd project list --workspace <confirmed-workspace-id>`.
5. Ask the user which project ID to use for disposable `[CFD-TEST]` entries.
6. Wait for explicit project confirmation.
7. Ensure no timer is running, or stop the current timer only after explicit user approval.
8. Create two disposable finished entries for the current day with a clear gap between them, for example:

   ```bash
   cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] timeline action source" --start <today 09:00> --end <today 10:00> --no-rounding -y
   cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] timeline edit blocker" --start <today 11:00> --end <today 12:00> --no-rounding -y
   ```

9. Create one disposable overnight entry that crosses local midnight, for example:

   ```bash
   cfd entry add --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] timeline overnight" --start <yesterday 23:30> --end <today 00:30> --no-rounding -y
   ```

10. Use a normal interactive terminal at least 80 columns wide and 24 rows tall.

## Steps

1. Run `cfd timeline --workspace <confirmed-workspace-id>`.
2. Confirm that the first screen uses all available vertical space for day timelines, with two timeline rows per day and the newest day at the bottom.
3. Confirm that the top row is a white shortcut bar with black text and does not contain `cfd timeline`.
4. Confirm that the top row shows navigation, reload, and quit shortcuts, and does not show `p` while no timer is running.
5. If older visible rows initially show `loading`, wait briefly and confirm they become loaded day rows without restarting the TUI.
6. Press `←` and `→`, and confirm the cursor moves while loading is in progress or after loading has completed.
7. Press `↑` repeatedly until the oldest visible loaded day is selected, then press `↑` once more. Confirm the selected day moves to the immediately previous calendar day, for example from Monday to Sunday, while another older week is scheduled without freezing the TUI. It must not jump to the same weekday one week earlier.
8. Press `↓`, `Home`, `End`, and `t`, and confirm navigation remains responsive. Confirm the `t` shortcut label is `now`.
9. Resize the terminal taller and confirm additional older rows appear and load in the background.
10. Move the cursor into a gap and confirm the bottom shortcut bar is not shown.
11. Press `n`, `s`, and `d` in the gap and confirm each key beeps and does not start, split, or delete anything.
12. Move the cursor onto the disposable `[CFD-TEST]` entry.
13. Confirm that the selected day's two timeline rows use a dark gray background across the full row, including the date and day total; the selected row's left date label, right duration total, and arrow markers remain white; and the bottom row appears as a colored shortcut bar with black text, uses the yellow current-entry highlight background, and shows only valid entry actions.
14. Confirm that `m`, `a`, and `e` appear only when the selected finished entry can be moved, its start moved, or its end moved by at least one rounding step in either direction. Confirm the bottom bar labels are `m move`, `a move start`, and `e move end`.
15. Press `m`, confirm the whole entry is highlighted, press `→`, and confirm start and end preview one step later. Press `Esc` and confirm the entry returns to its original time.
16. Press `a`, confirm the left edge is highlighted, press `→`, and confirm only the start preview moves later. Press `Enter` and confirm the timeline reloads with the shorter entry.
17. Press `e`, confirm the right edge is highlighted, press `→`, and confirm only the end preview moves later. Press `Enter` and confirm the timeline reloads with the longer entry.
18. Use `m`, `a`, or `e` to move an edge toward the second `[CFD-TEST]` entry until the next step would overlap. Confirm the blocked step beeps, leaves the preview unchanged, and does not show an inline overlap confirmation.
19. Confirm that `s` appears only when the cursor is at a valid split position inside the selected entry: at least one rounding step after the entry start and at least one rounding step before the entry end.
20. Press `s`, answer `y` to any inline overlap confirmation, and confirm the entry splits at the cursor time and the timeline reloads.
21. During a confirmation prompt, press an unrelated key such as `r` and confirm it beeps without reloading; then answer the prompt.
22. Move the cursor onto one of the resulting `[CFD-TEST]` entries.
23. Press `d`, answer `n`, and confirm the entry remains visible.
24. Press `d`, answer `y`, and confirm the entry disappears after reload.
25. Move the cursor onto the remaining `[CFD-TEST]` entry with project data.
26. Press `n`, answer `y` to any inline overlap confirmation, and confirm a running timer appears with the copied project/task/tags/description.
27. Move the cursor onto the running timer entry. Confirm that the bottom row shows `a move start` when the start can move by at least one rounding step, and does not show `m move`, `e move end`, `s split`, `n start`, or `d delete`.
28. Press `a`, confirm the left edge is highlighted, press `←` or `→`, and confirm only the start preview moves while the right edge continues to track `now`. Press `Esc` and confirm the timer returns to its original start.
29. Press `a` again, move the start by one valid step, press `Enter`, and confirm the timeline reloads with the timer still running and the changed start time.
30. Confirm that the top row now shows `p` for stopping the timer, and that the bottom row on a finished entry still shows `s` but no longer shows `n`.
31. Press `p`, answer `n`, and confirm the timer continues running.
32. Press `p`, answer `y`, and confirm the timer stops and the timeline reloads.
33. Reopen the TUI with `cfd timeline --workspace <confirmed-workspace-id> -y`, start a timer with `n` if needed, press `p`, and confirm the stop confirmation is skipped.
34. Press `r` and confirm the visible rows reload cleanly.
35. Move to yesterday's row and confirm the `[CFD-TEST] timeline overnight` entry appears from 23:30 to 24:00 and contributes 30 minutes to that day's total.
36. Move to today's row and confirm the same overnight entry appears from 00:00 to 00:30 and contributes 30 minutes to today's total.
37. Move the cursor onto either clipped overnight segment and confirm edit, split, and delete shortcuts are hidden. Confirm `n start` may still appear when no timer is running and the entry has project metadata.
38. Press `q` and confirm the terminal exits cleanly.
39. Stop and delete any remaining `[CFD-TEST]` timer or entry created by this journey.
40. Run `cfd timeline --workspace <confirmed-workspace-id> --format json`.
41. Run `cfd timeline --workspace <confirmed-workspace-id> --columns id`.

## Expected Results

- The TUI starts with as many two-row day timelines as fit in the terminal, not a fixed seven-day viewport.
- The date appears on the left and the day total appears on the right.
- Entry blocks and the vertical cursor render across both rows for each visible day.
- The first block row shows task or description when it fits, and the second block row shows duration when it fits.
- Entries that cross local midnight render as clipped segments on each affected day, and each day total includes only that day's segment duration.
- The top row is a white shortcut bar with black text and no `cfd timeline` title.
- The top row shows only currently available global shortcuts. The `p` shortcut is based on the current running timer status, not only on running entries visible in the loaded timeline rows.
- The selected day's two timeline rows use a dark gray background across the full row, including the date and day total; the selected row's left date label, right duration total, and arrow markers remain white.
- The bottom row appears only over entries with valid actions, uses the yellow current-entry highlight background with black text, and is blank over gaps or non-actionable entries.
- `m` and `e` appear only for finished loaded entries when at least one cursor-step edit is valid. `a` appears for finished loaded entries and for the running timer entry when the start can move by at least one cursor step.
- Edit, split, and delete shortcuts are hidden for clipped cross-midnight segments; starting a timer from the segment may still be available because it only copies metadata.
- Edit mode highlights the whole entry for move, the left edge for start adjustment, and the right edge for end adjustment.
- On a running timer entry, only `a move start` is available; moving the start keeps the timer running and leaves the right edge tracking `now`.
- Edit mode accepts `←` / `→` for previews, `Esc` for cancel, `Enter` for save, and `Ctrl-C` for exit; unrelated keys beep.
- Invalid edit steps, including overlaps and local-day boundary crossings, beep and leave the preview unchanged.
- Saved edit previews reload the timeline and persist exact previewed timestamps.
- The `s` split shortcut appears only for finished entries when the cursor is at a valid split position inside the entry.
- Hidden shortcuts emit a terminal bell and do not run.
- Confirmation prompts accept only displayed confirm/cancel shortcuts, with Ctrl-C still available to exit.
- Older visible days load in week-sized background batches.
- Empty days count as loaded rows and do not keep showing `loading`.
- Cursor and day navigation remain responsive while background loading is active.
- Scrolling above loaded rows schedules another older week without blocking.
- When navigating above the oldest loaded day, the selected day moves to the immediately previous calendar day while older days load in a week-sized batch.
- Reload ignores stale background responses and leaves the TUI usable.
- `n` starts a timer from the entry under the cursor when no timer is running, with inline overlap confirmation when needed.
- `n` is hidden and inactive while a timer is running; `s` remains available for finished entries.
- `p` asks for inline confirmation before stopping a running timer, and skips that confirmation when `-y` is passed.
- `s` splits the finished entry under the cursor at the cursor time, with inline overlap confirmation when needed.
- `d` asks for inline confirmation; `n` cancels and `y` deletes.
- Successful actions reload the timeline and leave it usable.
- `q` restores the terminal.
- `--format` and `--columns` both fail clearly because `timeline` has no machine-readable output.

## Cleanup

Stop and delete all `[CFD-TEST]` timers and entries created during setup or action testing.
