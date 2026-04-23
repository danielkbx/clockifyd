# 06 Rounding and Overlaps

## Goal

Verify configured rounding, `--no-rounding`, and overlap confirmation.

## Preconditions

- Confirmed workspace
- Valid project ID

## Steps

1. Set rounding to `15m`
2. Add an entry with a timestamp that should round
3. Verify the stored entry uses rounded timestamps
4. Add or update another entry that overlaps the first one
5. Confirm that the CLI warns before writing
6. Repeat the overlap case with `-y`
7. Repeat a create or update case with `--no-rounding`
8. Unset rounding

## Expected Results

- Rounding applies when configured
- `--no-rounding` bypasses rounding for that invocation
- Overlaps produce a warning and require confirmation unless `-y` is used
- Future timestamps caused by rounding are accepted

## Cleanup

- Delete all entries created by the journey
- Unset rounding if still set
