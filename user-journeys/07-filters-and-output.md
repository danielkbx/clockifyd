# 07 Filters and Output

## Goal

Verify list filters and output modes.

## Preconditions

- Confirmed workspace
- At least one entry exists in the chosen time range

## Steps

1. Run `cfd entry list --start <iso> --end <iso> --format text`
2. Run the same command with `--format json`
3. Run the same command with `--no-meta`
4. Run `cfd entry list --start today --end today`
5. Run `cfd entry list --start yesterday --end yesterday`
6. If possible, filter by `--project`
7. If possible, filter by `--task`
8. If possible, filter by `--tag`
9. Run `cfd entry text list --project <project-id>`
10. Run `cfd entry text list --project <project-id> --no-meta`

## Expected Results

- Text output is compact and readable
- JSON output is valid JSON
- `--no-meta` suppresses metadata fields in text mode
- `today` and `yesterday` resolve correctly in the local timezone
- `entry text list` returns deduplicated descriptions ordered by latest use
- `entry text list --no-meta` prints one description per line
- Filters narrow the result set correctly

## Cleanup

- None
