# 13 Timer Aliases

## Goal

Verify local timer aliases in an isolated config file:

- `alias create`
- `alias list`
- `alias delete`
- dynamic `cfd <alias> start`
- ytd-style interactive default prompt rendering

## Preconditions

- Confirmed workspace
- Confirmed project in that workspace
- Optional confirmed task in that project
- Permission to start and stop a timer
- No timer is running before the journey starts, or permission to stop the existing timer first

## Setup

Use a temporary config file so aliases and defaults do not modify the user's normal config:

```bash
ALIAS_CONFIG=$(mktemp /tmp/cfd-aliases.XXXXXX.json)
```

Populate `$ALIAS_CONFIG` with a valid API key. If the current login uses the default config file, copy it as a starting point:

```bash
cp ~/.config/cfd/config.json "$ALIAS_CONFIG"
```

If auth comes from `CLOCKIFY_API_KEY`, write a minimal config or keep using the env var with `CFD_CONFIG="$ALIAS_CONFIG"`.

Expected: `env CFD_CONFIG="$ALIAS_CONFIG" cfd whoami` succeeds.

## Steps

1. Run `cfd workspace list`.
2. Ask the user to confirm which workspace to use.
3. Run `cfd project list --workspace <confirmed-workspace-id>`.
4. Ask the user to confirm which project to use.
5. Optional: run `cfd task list --workspace <confirmed-workspace-id> --project <confirmed-project-id>` and ask whether to use a task.
6. Create a project-only alias:
   `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create quick --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] alias quick"`
7. Verify stored config:
   `jq '.aliases.quick' "$ALIAS_CONFIG"`
8. List aliases as text:
   `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias list --workspace <confirmed-workspace-id>`
9. List aliases as JSON and raw, then compare:
   `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias list --workspace <confirmed-workspace-id> --format json > /tmp/cfd-alias-json.out`
   `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias list --workspace <confirmed-workspace-id> --format raw > /tmp/cfd-alias-raw.out`
   `diff -u /tmp/cfd-alias-json.out /tmp/cfd-alias-raw.out`
10. If a task was confirmed, update the alias with task:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create quick --workspace <confirmed-workspace-id> --task <confirmed-task-id>`
11. Verify omitted fields were preserved:
    `jq '.aliases.quick' "$ALIAS_CONFIG"`
12. Clear optional fields:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create quick --workspace <confirmed-workspace-id> --task none --description none`
13. Verify `task` and `description` are absent:
    `jq '.aliases.quick' "$ALIAS_CONFIG"`
14. Recreate the alias for runtime start:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create quick --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] alias timer"`
15. Start a timer through the alias:
    `ALIAS_ENTRY_ID=$(env CFD_CONFIG="$ALIAS_CONFIG" cfd quick start --workspace <confirmed-workspace-id> --no-rounding -y)`
16. Run:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd timer current --workspace <confirmed-workspace-id>`
17. Stop the timer:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd timer stop --workspace <confirmed-workspace-id> --no-rounding -y`
18. Capture the stopped entry ID from the stop output when possible.
19. Delete the alias:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias delete quick -y`
20. Confirm deleted dynamic alias fails:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd quick start --workspace <confirmed-workspace-id>`
21. Interactive prompt check in a terminal:
    - Run `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create promptcheck --workspace <confirmed-workspace-id> --project <confirmed-project-id> --description "[CFD-TEST] prompt default"`
    - Run `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias create promptcheck --workspace <confirmed-workspace-id>` without `--project`, `--task`, or `--description`
    - Press Enter through the prompts to accept defaults
    - Observe prompt text before pressing Enter
22. Delete `promptcheck`:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd alias delete promptcheck -y`
23. Delete the time entry created by this journey if it still exists:
    `env CFD_CONFIG="$ALIAS_CONFIG" cfd entry delete <created-entry-id> --workspace <confirmed-workspace-id> -y`
24. Remove temporary files:
    `rm -f "$ALIAS_CONFIG" /tmp/cfd-alias-json.out /tmp/cfd-alias-raw.out`

## Expected Results

- `alias create` prints only the alias name.
- Stored aliases live under the config `aliases` key.
- Alias config stores IDs only for `project` and `task`.
- `alias list` text output shows alias name and project/task/description fields.
- `alias list --format json` and `--format raw` are equivalent.
- Updating an alias without `--description` preserves the existing description.
- `--task none` removes the stored `task` key.
- `--description none` removes the stored `description` key.
- `cfd quick start` starts a running timer using the alias project and description.
- `timer current` shows the alias-created running timer.
- `timer stop` stops the timer successfully.
- Deleted aliases are no longer available as dynamic commands.
- Interactive defaults match ytd-style label rendering:
  - `Select Project [<project name>]:`
  - `Select Task [none]:` or `Select Task [<task name>]:`
  - `Description [[CFD-TEST] prompt default]:`
- Interactive defaults do not show numeric default indexes such as `Select Project [1]:`.
- Interactive defaults do not include `(default)`.

## Cleanup

- Stop any timer started by this journey.
- Delete any time entry created by this journey.
- Delete `quick` and `promptcheck` aliases if they still exist.
- Remove `$ALIAS_CONFIG`.
- Remove `/tmp/cfd-alias-json.out` and `/tmp/cfd-alias-raw.out`.
