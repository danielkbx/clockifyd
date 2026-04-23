# 05 Task Create

## Goal

Verify explicit task creation for ticket-like names.

## Preconditions

- Confirmed workspace
- Valid project ID with task creation permissions

## Steps

1. Run `cfd task create --project <project-id> --name "[CFD-TEST] ABC-1: Implement something nice"`
2. Capture the returned task ID
3. Run `cfd task get <project-id> <task-id>`
4. Run `cfd task list --project <project-id>` and verify the task appears

## Expected Results

- Create returns only the task ID
- Get returns the created task with the expected name
- List includes the new task

## Cleanup

- Remove the test task if the implemented workflow later supports deletion in journeys
