# 04 Project Client Task Tag Browse

## Goal

Verify metadata browse commands.

## Preconditions

- Confirmed workspace
- At least one project available

## Steps

1. Run `cfd project list`
2. Pick a valid project ID from the output
3. Run `cfd project get <project-id>`
4. Run `cfd client list`
5. If clients exist, run `cfd client get <client-id>`
6. Run `cfd tag list`
7. If tags exist, run `cfd tag get <tag-id>`
8. Run `cfd task list --project <project-id>`
9. If tasks exist, run `cfd task get <project-id> <task-id>`

## Expected Results

- List commands return compact output
- Get commands return the referenced entity

## Cleanup

- None
