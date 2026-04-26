# 10 Agent Skill Generation

## Goal

Verify that generated agent skill guidance is useful and current without requiring exact wording.

## Preconditions

- `cfd` is built and available
- Generic skill checks require no login
- Workspace-specific checks require a confirmed workspace and valid API key

## Steps

1. Run `cfd skill --scope brief`
2. Review the output as Markdown, not exact text
3. Run `cfd skill --scope standard`
4. Run `cfd skill --scope full`
5. Compare the three outputs
6. Run `cfd skill --format md --scope standard`
7. Run `cfd skill --format json` and confirm it fails
8. Run `cfd skill --scope invalid` and confirm it fails
9. Confirm no credentials are needed for generic output by running with an isolated missing `CFD_CONFIG` and no `CLOCKIFY_API_KEY`

## Expected Results

- Output starts with valid skill frontmatter
- Description includes `time tracking`
- Description or body distinguishes Clockify time tracking from generic issue tracker work logs
- Keeping-current instructions include `cfd --version`
- Keeping-current instructions include a regeneration command using the effective scope
- `brief` is shorter and omits detailed recipes/reference
- `standard` includes workflow, examples, recipes, and safety guidance
- `full` includes the full command reference and detailed troubleshooting/config guidance
- `--format md` prints Markdown successfully
- `--format json` and invalid scope fail before config/auth is needed

## Cleanup

- Remove any temporary config file
- Unset temporary env vars
