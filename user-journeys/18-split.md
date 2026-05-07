# Split Entries And Timers

Verify finished-entry split, running-timer split, gap rounding order, no-rounding behavior, overlap prompts, and active-switch state preservation.

## Preconditions

- Run `cfd workspace list` and ask the user to confirm the workspace ID.
- Run `cfd project list --workspace <confirmed-workspace-id>` and ask the user to confirm a project ID.
- Use `CFD_CONFIG` for config isolation when changing rounding defaults.
- Use `[CFD-TEST]` in every temporary description.
- Confirm no important timer is running before starting timer split checks.

## Finished Entry Split

1. Create a temporary manual entry:
   `cfd entry add --workspace <workspace-id> --start <start> --duration 2h --project <project-id> --description "[CFD-TEST] split entry"`
2. Capture the created entry ID.
3. Run:
   `cfd split entry --workspace <workspace-id> <entry-id> --at <start+1h> --format json --no-rounding`
4. Confirm JSON has both `updated` and `created`.
5. Confirm `updated.id` is the original entry ID.
6. Confirm `updated.timeInterval.end` equals the split time.
7. Confirm `created.timeInterval.start` equals the split time.
8. Confirm `created.timeInterval.end` equals the original old end.
9. Confirm project, task when present, tags when present, and description were copied.

## Gap And Rounding Order

1. Set isolated rounding to `15m`:
   `cfd config set rounding 15m`
2. Create another `[CFD-TEST]` two-hour manual entry with clean timestamps.
3. Run:
   `cfd split entry --workspace <workspace-id> <entry-id> --at <10:07 timestamp> --gap 5m --format json`
4. Confirm the split end is the rounded `--at` value.
5. Confirm the new start was calculated by adding `5m` to the rounded split end, then rounding that calculated value again.
6. Repeat with `--no-rounding` and confirm the new start is exactly unresolved split time plus `5m`.
7. Restore prior rounding config or unset the isolated config.

## Running Timer Split

1. Start a timer:
   `cfd timer start "[CFD-TEST] split timer" --workspace <workspace-id> --project <project-id> --start <past-start> --no-rounding`
2. Run:
   `cfd split timer --workspace <workspace-id> --at <past-start+15m> --gap 5m --format json --no-rounding`
3. Confirm JSON has both `updated` and `created`.
4. Confirm `updated.id` is the stopped original timer.
5. Confirm `updated.timeInterval.end` equals the split time.
6. Confirm `created.timeInterval.start` equals split time plus gap.
7. Run `cfd timer current --workspace <workspace-id> --format json`.
8. Confirm the current running timer ID is `created.id` and fields were copied.
9. Stop the timer and clean up all `[CFD-TEST]` entries.

## Overlap Prompt

1. Create two temporary entries that will overlap the two intervals produced by a split.
2. Run `cfd split entry ...` without `-y`.
3. Confirm stderr warns once with the overlapping entry IDs and prompts for confirmation.
4. Abort the prompt and confirm no split mutation happened.
5. Run the same command with `-y`.
6. Confirm the warning still appears and the prompt is skipped.
7. Clean up temporary entries.

## Active Switch Timer Split

1. Start Timer A with `[CFD-TEST] split switch A`.
2. Run `cfd switch start "[CFD-TEST] split switch B" --workspace <workspace-id> --project <project-id> --start <switch-start> --no-rounding`.
3. Run `cfd split timer --workspace <workspace-id> --at <switch-start+10m> --format json --no-rounding`.
4. Run `cfd switch current --workspace <workspace-id> --format json`.
5. Confirm `current.id` is the `created.id` from the split result.
6. Confirm `returnsTo` still describes Timer A.
7. Run `cfd switch stop --workspace <workspace-id> --no-rounding`.
8. Confirm Timer A resumes and clean up all `[CFD-TEST]` entries.

## Expected Results

- Entry split updates the original entry and creates a copied second entry.
- Timer split stops the old timer and starts a copied new timer.
- Gap is added to the already rounded split/end timestamp.
- The calculated new start is rounded again unless `--no-rounding` is present.
- Overlap detection runs for both resulting intervals and `-y` skips only prompts.
- Active switch timer split keeps switch state usable.
