# Temporary Switches

Verify temporary timer switching and return-target visibility.

## Preconditions

- Confirm a workspace through the standard journey process.
- Confirm one project for Timer A and one project or alias target for Timer B.
- Use `CFD_CONFIG` when config isolation is needed.

## Steps

1. Run `cfd timer start "CFD journey Timer A" --workspace <workspace-id> --project <project-a-id>`.
2. Run `cfd switch start "CFD journey Timer B" --workspace <workspace-id> --project <project-b-id>`.
3. Run `cfd switch current --workspace <workspace-id> --format json`.
4. Confirm `current` describes Timer B.
5. Confirm `returnsTo` describes Timer A, including `originalEntryId`, `switchedAt`, project, task when present, tags when present, and description.
6. Run `cfd status --workspace <workspace-id> --format json`.
7. Confirm the status timer object includes the return target while Timer B is running.
8. Run `cfd switch stop --workspace <workspace-id>`.
9. Run `cfd timer current --workspace <workspace-id> --format json`.
10. Confirm the running timer has Timer A's project/task/tags/description.
11. Start Timer A again, then run `cfd timer switch resume -1 --workspace <workspace-id> -y`.
12. Confirm `cfd switch current --workspace <workspace-id> --format json` shows the selected recent entry under `current` and Timer A under `returnsTo`.
13. Stop and clean up any entries created by the journey.

## Alias Variant

1. Create or reuse a temporary alias with `cfd alias create <alias> --project <project-b-id> --description "CFD journey alias switch"`.
2. Start Timer A.
3. Run `cfd <alias> switch --workspace <workspace-id>`.
4. Run `cfd switch current --workspace <workspace-id> --format json`.
5. Confirm `current` uses the alias fields and `returnsTo` describes Timer A.
6. Stop the switch and clean up entries and alias.

## Expected Results

- Switch boundaries produce contiguous time entries without visible gaps.
- `switch current` exposes both `current` and `returnsTo`.
- `status` exposes the return target while a switch is active.
- `timer stop` during an active switch should behave like `switch stop`.
- Too-short switched timers are discarded and the original timer resumes from the original switch time.
