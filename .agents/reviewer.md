# Code Review Standards

## Non-negotiable

- Core must not import CLI command modules
- API keys must never appear in logs, errors, or stdout
- Create/update commands print only the resource ID on stdout
- Errors go to stderr, exit code non-zero
- `--format` and `--no-meta` must be respected by every command
- `--format json` is the documented machine format; `raw` may only remain as compatibility alias

## CLI Entry Point

- Known-command validation must happen before config loading
- Help routing must support both `cfd help <cmd>` and `cfd <cmd> help`
- Interactive login must validate credentials by loading workspaces before saving config

## Output

- Default text output should be line-based and readable without column alignment assumptions
- List commands in default text mode should separate items with blank lines
- `entry list|get --columns` must switch to one-row-per-entry tab-separated output with no header
- Bare `--columns` must fail clearly
- `--columns` and `--format` must be mutually exclusive

## Rounding

- Rounding must affect only mutating time commands
- `--no-rounding` must reliably disable active rounding for one invocation
- Post-rounding invalid intervals must fail clearly
- Future timestamps produced by rounding are allowed and must not be rejected

## Overlaps

- Overlap checks apply only within the same workspace and current user context
- `entry update` must exclude its own current entry from collision checks
- Overlap is a warning + confirmation, not a hard rejection
- `-y` may skip the confirmation but must not skip the detection itself

## Scope

- Changes should touch only what the task requires
- No speculative abstractions
- No unrelated cleanup

## Report Format

```text
BLOCKER: <issue>
WARN: <issue>
OK
```
