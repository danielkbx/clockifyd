# 16 Relative Datetime Inputs

## Goal

Verify relative `--start` and `--end` inputs across manual entries, list filters, entry updates, timers, and rounding-sensitive flows.

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Permission to create, update, stop, and delete time entries
- No timer is running before timer-specific steps, or permission to stop the current timer first
- Test runner knows the current local time well enough to verify relative windows
- Use `--no-rounding` unless the step explicitly verifies rounding interaction

## Setup Rules

1. Run `cfd workspace list`.
2. Ask the user to confirm which workspace to use.
3. Run `cfd project list --workspace <confirmed-workspace-id>`.
4. Ask the user to confirm which project to use.
5. Track every created entry ID for cleanup.
6. Prefer descriptions prefixed with `[CFD-TEST] relative datetime`.

## Scenario A: Manual Entry Add Relative To Now

1. Create an entry:

   ```bash
   ENTRY_ID=$(cfd entry add --workspace <workspace-id> --project <project-id> --start -45m --duration 45m --description "[CFD-TEST] relative datetime add duration" --no-rounding -y)
   ```

2. Get the entry:

   ```bash
   cfd entry get "$ENTRY_ID" --workspace <workspace-id>
   ```

3. Verify:
   - create returns only the entry ID on stdout
   - start is approximately 45 minutes before the create command ran
   - end is approximately when the create command ran
   - description is `[CFD-TEST] relative datetime add duration`
   - project matches the confirmed project

Allow a small execution delay when comparing against current time, for example 2 minutes.

## Scenario B: Manual Entry Add With Relative Start And End

1. Create an entry:

   ```bash
   ENTRY_ID=$(cfd entry add --workspace <workspace-id> --project <project-id> --start now-2h --end now-90m --description "[CFD-TEST] relative datetime add interval" --no-rounding -y)
   ```

2. Get the entry:

   ```bash
   cfd entry get "$ENTRY_ID" --workspace <workspace-id>
   ```

3. Verify:
   - interval duration is about 30 minutes
   - raw relative strings such as `now-2h` and `now-90m` are not shown as stored values
   - stored start and end are concrete timestamps
   - description and project match the created entry

## Scenario C: Entry List Relative Window

1. List entries in a relative window:

   ```bash
   cfd entry list --workspace <workspace-id> --start -3h --end now+5m --text "[CFD-TEST] relative datetime" --columns start,end,description --sort asc
   ```

2. Verify:
   - entries from Scenario A and Scenario B appear
   - output uses concrete stored timestamps
   - rows are sorted ascending by start time
   - no entry outside the relative window is required for success

## Scenario D: Entry Update Bare Relative Existing Field

1. Create a stable base entry:

   ```bash
   ENTRY_ID=$(cfd entry add --workspace <workspace-id> --project <project-id> --start now-3h --duration 1h --description "[CFD-TEST] relative datetime update base" --no-rounding -y)
   ```

2. Capture current start and end:

   ```bash
   cfd entry get "$ENTRY_ID" --workspace <workspace-id> --columns start,end
   ```

3. Move the existing end 5 minutes earlier:

   ```bash
   cfd entry update "$ENTRY_ID" --workspace <workspace-id> --end -5m --no-rounding -y
   ```

4. Get the entry again and verify:
   - start is unchanged
   - end is exactly 5 minutes earlier than the previous stored end, allowing only timestamp formatting differences
   - project and description are unchanged

5. Move the existing start 10 minutes later:

   ```bash
   cfd entry update "$ENTRY_ID" --workspace <workspace-id> --start +10m --no-rounding -y
   ```

6. Get the entry again and verify:
   - start is 10 minutes later than the previous stored start
   - end is unchanged from the prior updated end
   - interval remains valid

## Scenario E: Entry Update `now-...` Means Now, Not Existing Field

1. Create an entry:

   ```bash
   ENTRY_ID=$(cfd entry add --workspace <workspace-id> --project <project-id> --start now-2h --duration 30m --description "[CFD-TEST] relative datetime update now base" --no-rounding -y)
   ```

2. Update the end relative to current time:

   ```bash
   cfd entry update "$ENTRY_ID" --workspace <workspace-id> --end now-5m --no-rounding -y
   ```

3. Get the entry and verify:
   - end is approximately five minutes before the update command ran
   - end is not interpreted as five minutes before the previously stored end
   - start remains unchanged

## Scenario F: Running Entry Rejects Bare Relative End

1. Start a timer:

   ```bash
   TIMER_ID=$(cfd timer start "[CFD-TEST] relative datetime running update" --workspace <workspace-id> --project <project-id> --start -10m --no-rounding -y)
   ```

2. Try to adjust the missing end:

   ```bash
   cfd entry update "$TIMER_ID" --workspace <workspace-id> --end -5m --no-rounding -y
   ```

3. Verify:
   - command exits non-zero
   - stderr says the missing end cannot be adjusted
   - stderr suggests `--end now-5m` or `--duration <d>`

4. Stop the timer for cleanup:

   ```bash
   cfd timer stop --workspace <workspace-id> --end now --no-rounding -y
   ```

## Scenario G: Timer Start And Stop Relative Times

1. Start a timer in the past:

   ```bash
   TIMER_ID=$(cfd timer start "[CFD-TEST] relative datetime timer" --workspace <workspace-id> --project <project-id> --start -10m --no-rounding -y)
   ```

2. Verify current timer start is approximately 10 minutes ago:

   ```bash
   cfd timer current --workspace <workspace-id>
   ```

3. Stop the timer:

   ```bash
   cfd timer stop --workspace <workspace-id> --end now --no-rounding -y
   ```

4. Get the stopped entry when possible and verify:
   - stopped entry has concrete start and end timestamps
   - duration is approximately 10 minutes
   - description is `[CFD-TEST] relative datetime timer`

## Scenario H: Rounding Interaction

1. Set rounding:

   ```bash
   cfd config set rounding 15m
   ```

2. Add an entry without `--no-rounding`:

   ```bash
   ENTRY_ID=$(cfd entry add --workspace <workspace-id> --project <project-id> --start -17m --duration 10m --description "[CFD-TEST] relative datetime rounding" -y)
   ```

3. Get the entry and verify:
   - relative start was accepted
   - stored start and end are rounded to configured 15-minute boundaries
   - description is `[CFD-TEST] relative datetime rounding`

4. Unset rounding:

   ```bash
   cfd config unset rounding
   ```

## Scenario I: Invalid Relative Inputs

Verify each command exits non-zero before creating or updating data:

```bash
cfd entry add --workspace <workspace-id> --project <project-id> --start 15m --duration 10m --description "[CFD-TEST] should fail"
cfd entry add --workspace <workspace-id> --project <project-id> --start -2d --duration 10m --description "[CFD-TEST] should fail"
cfd entry list --workspace <workspace-id> --start nowish --end now
cfd timer start "[CFD-TEST] should fail" --workspace <workspace-id> --project <project-id> --start -
```

Expected results:

- clear `invalid start` or `invalid end` messages
- no new entries are created
- no timer remains running

## Expected Results

- Relative values are accepted for all documented `--start` and `--end` flags.
- `now` and `now+duration` or `now-duration` are always relative to current time.
- Bare `+duration` or `-duration` in `entry update` adjusts the existing field.
- Bare `+duration` or `-duration` in other commands is relative to current time.
- Running entries reject bare relative `--end` updates when no stored end exists.
- Rounding applies after relative values are resolved.
- Stored and listed Clockify values are concrete timestamps, not relative strings.
- Invalid relative values fail clearly.

## Cleanup

- Stop any `[CFD-TEST] relative datetime` timer still running.
- Delete every created `[CFD-TEST] relative datetime` entry.
- Unset rounding if the journey changed it.
- If cleanup cannot identify an entry ID, list candidate entries:

  ```bash
  cfd entry list --workspace <workspace-id> --start -24h --end now --text "[CFD-TEST] relative datetime"
  ```
