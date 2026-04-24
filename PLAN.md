# PLAN: Shell Completion Support

Issue: https://github.com/danielkbx/clockifyd/issues/1

Goal: add first-party `cfd completion bash|zsh|fish` support that generates static shell completion scripts from a canonical, repo-owned CLI model.

Execution model:

- Execute one step at a time.
- Each step is intended for a fresh subagent with only this file, `AGENTS.md`, and the listed source files as starting context.
- After finishing a step, the subagent must update this file by changing that step from `[ ]` to `[x]` and filling in the Handover section.
- Each subagent should run only the tests listed for its step unless local changes make broader verification necessary.
- Do not revert unrelated existing changes in the worktree.

Global constraints:

- Keep core modules free of CLI command dependencies.
- Generated scripts must be written to stdout only.
- Unsupported completion shells must return a clear error through the normal stderr path.
- Avoid dynamic Clockify API completions in this implementation.
- Keep the canonical spec focused on user-visible CLI structure; it is not a runtime parser.

Definition of done:

- `cfd completion bash`, `cfd completion zsh`, and `cfd completion fish` produce non-empty shell-specific scripts.
- The completion scripts include current commands, subcommands, global flags, key command flags, and fixed enum values.
- `cfd help completion` documents the command.
- README contains setup instructions for Bash, Zsh, and Fish.
- Tests fail if the known command tree or fixed enum values drift away from completion support.
- `cargo fmt` and `cargo test` pass.

## Step 1: Add Canonical CLI Spec

Status: [x]

Objective:

Create a small internal model of the visible CLI surface that completion renderers and drift tests can consume.

Primary files:

- `src/cli_spec.rs`
- `src/main.rs`
- `src/args.rs`

Implementation details:

- Add `mod cli_spec;` in `src/main.rs`.
- Define lightweight structs such as:
  - `CommandSpec`
  - `OptionSpec`
  - `PositionalSpec`
- Include enough fields for completion/documentation:
  - command name
  - short help/about text
  - nested subcommands
  - options/flags
  - positionals
  - value name
  - repeatability
  - fixed possible values
- Add shared fixed-value constants:
  - `FORMAT_VALUES = ["text", "json", "raw"]`
  - `ROUNDING_VALUES = ["off", "1m", "5m", "10m", "15m"]`
  - completion shells: `["bash", "zsh", "fish"]`
- Model all currently visible commands:
  - `help`
  - `login`
  - `logout`
  - `whoami`
  - `workspace list|get`
  - `config`, `config interactive`, `config set|get|unset workspace|project|rounding`
  - `project list|get`
  - `client list|get`
  - `tag list|get`
  - `task list|get|create`
  - `entry list|get|add|update|delete`
  - `entry text list`
  - `timer current|start|stop`
  - `completion bash|zsh|fish`
- Model global flags:
  - `--format <text|json|raw>`
  - `--no-meta`
  - `--workspace <id>`
  - `--no-rounding`
  - `-y`
  - `--version`
- Model common command-specific options:
  - `--columns`
  - `--project`
  - `--task`
  - `--tag`
  - `--text`
  - `--start`
  - `--end`
  - `--duration`
  - `--name`
  - `--description`
- Do not change runtime parsing behavior in this step unless a compile error requires a tiny adjustment.

Tests:

- Add unit tests in `src/cli_spec.rs` covering:
  - all top-level commands exist
  - `completion` has `bash`, `zsh`, and `fish`
  - global flags include expected long/short names
  - `--format` values match `FORMAT_VALUES`
  - `config set rounding` exposes `ROUNDING_VALUES`

Verification:

- Run `cargo test cli_spec`.

Handover:

- Summary: Added `src/cli_spec.rs` with lightweight command, option, and positional specs; modeled the current visible CLI tree, global flags, common command options, fixed format/rounding values, and completion shells. Registered the module in `src/main.rs` without changing runtime parsing or routing.
- Files changed: `src/cli_spec.rs`, `src/main.rs`, `PLAN.md`
- Tests run: `cargo fmt`; `cargo test cli_spec`
- Notes for next step: `cli_spec` intentionally has `#![allow(dead_code)]` because Step 1 introduces the model before Step 2/renderer code consumes it. Step 2 can add traversal helpers for command paths and parser/spec drift checks.

## Step 2: Add Drift Tests Against Router and Parser

Status: [x]

Objective:

Make the canonical CLI spec hard to forget when commands or key flags change.

Primary files:

- `src/main.rs`
- `src/args.rs`
- `src/cli_spec.rs`

Implementation details:

- Expose a test-only or normal helper from `cli_spec.rs` that returns all known command paths as tuples or strings, for example:
  - `login`
  - `workspace list`
  - `entry text list`
- Add tests in `src/main.rs` that assert every command path in the CLI spec is accepted by `is_known_command`, except special cases that are not routed as normal commands:
  - `help`
  - `--version`
- Add or adjust tests to include `completion bash|zsh|fish` once Step 3 wires it into `is_known_command`.
- Add tests that ensure documented value-taking flags from the parser are represented in the spec:
  - `format`
  - `workspace`
  - `columns`
  - `project`
  - `start`
  - `end`
  - `text`
  - `task`
  - `tag`
  - `duration`
  - `description`
  - `name`
- If `args::takes_value` must become visible for testing, prefer `pub(crate)` over broader exposure.
- Keep this step test-focused; do not implement renderers yet.

Tests:

- Router/spec drift tests.
- Parser/spec flag drift tests.

Verification:

- Run `cargo test main::`.
- Run `cargo test args::`.
- Run `cargo test cli_spec`.

Handover:

- Summary: Added CLI-spec traversal helpers for leaf command paths and recursive long option names; added router drift coverage for spec command paths that are currently routable; added parser/spec drift coverage ensuring every value-taking parser flag is represented in the canonical CLI spec. Completion paths are intentionally asserted as pending Step 3 router work.
- Files changed: `src/cli_spec.rs`, `src/main.rs`, `src/args.rs`, `PLAN.md`
- Tests run: `cargo test main::`, `cargo test args::`, `cargo test cli_spec`, `cargo test cli_spec_routable_command_paths_are_known_commands`, `cargo test completion_paths_remain_step_three_router_work`
- Notes for next step: Step 3 should update `is_known_command` so `completion bash|zsh|fish` become accepted, then invert/remove the temporary `completion_paths_remain_step_three_router_work` assertion and add the planned help/routing tests.

## Step 3: Wire Completion Command Routing

Status: [x]

Objective:

Make `completion` a recognized command path.

Primary files:

- `src/main.rs`
- `src/help.rs`
- `src/cli_spec.rs`

Implementation details:

- Update `is_known_command` to accept:
  - `completion bash`
  - `completion zsh`
  - `completion fish`
- For this step, the command may call a small placeholder function or simple temporary renderer only if needed to compile. The full shell renderers belong to Step 4.
- Unsupported shells should remain unknown-command errors or clear completion errors; after Step 4 they should use the final completion error path.
- Add global help entry:
  - `completion <bash|zsh|fish>  Generate shell completions`
- Add `cfd help completion` output with:
  - usage: `cfd completion <bash|zsh|fish>`
  - statement that generation writes the script to stdout
- Support `cfd completion help` if it fits the existing help routing cleanly.

Tests:

- `src/main.rs` unit tests:
  - completion shell paths are known
  - invalid shell path is rejected
- `src/help.rs` unit tests:
  - global help mentions completion
  - completion help includes Bash, Zsh, Fish, and stdout wording

Verification:

- Run `cargo test main::`.
- Run `cargo test help::`.

Handover:

- Summary: Wired `completion bash|zsh|fish` into known-command validation and top-level routing. Added a tiny stdout placeholder for valid shells so the command path executes until Step 4 replaces it with real renderers. Added completion command help and global help entry, plus unit coverage for valid/invalid completion paths and completion help wording.
- Files changed: `src/main.rs`, `src/help.rs`, `PLAN.md`
- Tests run: `cargo test main::`, `cargo test help::`
- Notes for next step: Step 4 should replace the placeholder `# cfd {shell} completion placeholder` branch in `src/main.rs` with the real renderer API/output and final unsupported-shell error behavior.

## Step 4: Implement Shell Renderers

Status: [x]

Objective:

Render static Bash, Zsh, and Fish completion scripts from the canonical CLI spec.

Primary files:

- `src/completion.rs`
- `src/main.rs`
- `src/cli_spec.rs`

Implementation details:

- Add `mod completion;` in `src/main.rs`.
- Implement an API such as:
  - `render_completion(shell: &str, spec: &CommandSpec) -> Result<String, CfdError>`
- Bash output requirements:
  - contains a `_cfd` completion function
  - registers with `complete -F _cfd cfd`
  - suggests top-level commands
  - includes global options and known command-specific options
  - includes fixed values for `--format` and rounding where practical
- Zsh output requirements:
  - contains a `_cfd` function or equivalent zsh completion structure
  - includes `#compdef cfd`
  - suggests top-level commands/subcommands/options
  - includes fixed values for `--format` and rounding where practical
- Fish output requirements:
  - uses `complete -c cfd ...` lines
  - includes top-level commands/subcommands/options
  - includes fixed values for `--format` and rounding where practical
- Keep implementation straightforward and static.
- Escape generated shell strings safely enough for current command/flag/value names.
- Do not add external dependencies.

Tests:

- Unit tests in `src/completion.rs`:
  - Bash output is non-empty and contains `_cfd`, `complete -F _cfd cfd`, `workspace`, `entry`, `--format`, `json`, `--workspace`
  - Zsh output is non-empty and contains `#compdef cfd`, `_cfd`, `workspace`, `entry`, `--format`, `json`
  - Fish output is non-empty and contains `complete -c cfd`, `workspace`, `entry`, `--format`, `json`
  - rounding values appear in generated output
  - unsupported shell returns an error

Verification:

- Run `cargo test completion::`.

Handover:

- Summary: Added `src/completion.rs` with static Bash, Zsh, and Fish renderers driven by the canonical CLI spec. The renderers include command/subcommand names, collected global/command options, `--format` fixed values, and `config set rounding` fixed values. Added unsupported-shell error handling and renderer unit tests. Registered the module in `src/main.rs` for compilation only; the runtime command still uses the Step 3 placeholder for Step 5 to connect.
- Files changed: `src/completion.rs`, `src/main.rs`, `PLAN.md`
- Tests run: `cargo test completion::`
- Notes for next step: Step 5 should replace the placeholder completion branch in `src/main.rs` with `completion::render_completion(shell, &cli_spec::cli_spec())`, print the generated script with exactly one trailing newline, and add CLI integration tests for the generated scripts.

## Step 5: Connect Runtime Command to Final Renderers

Status: [x]

Objective:

Make `cfd completion bash|zsh|fish` execute the final renderer and print only the generated script.

Primary files:

- `src/main.rs`
- `src/completion.rs`
- optionally `src/commands/completion.rs`

Implementation details:

- Route `("completion", Some(shell), None)` to the renderer.
- Print renderer output to stdout with one trailing newline if the renderer did not already include one.
- Return `Ok(())` on supported shells.
- Return `CfdError::message(...)` on unsupported shell if that path can be reached.
  - `ClockifyClient::new`
- Decide whether to use `commands/completion.rs`:
  - Use it if routing in `main.rs` starts to feel bulky.
  - Keep it small.

Tests:

- CLI integration tests, likely new file `tests/completion_cli.rs`:
  - `cfd completion bash` succeeds and emits Bash-specific markers
  - `cfd completion zsh` succeeds and emits Zsh-specific markers
  - `cfd completion fish` succeeds and emits Fish-specific markers
  - outputs include representative commands and flags
  - command succeeds for all supported shells
  - invalid shell exits non-zero with a useful stderr message

Verification:

- Run `cargo test --test completion_cli`.
- Run `cargo test completion::`.

Handover:

- Summary: Replaced the Step 3 placeholder runtime branch with `completion::render_completion(shell, &cli_spec::cli_spec())` and ensured stdout gets exactly one trailing newline. Added CLI integration coverage for Bash, Zsh, Fish, representative generated markers, and unsupported shell errors.
- Files changed: `src/main.rs`, `tests/completion_cli.rs`, `PLAN.md`
- Tests run: `cargo test --test completion_cli`; `cargo test completion::`
- Notes for next step: Step 6 should verify README/help wording against the final behavior. Runtime invalid shells now reach the renderer and return `unsupported completion shell: <shell>` even though `is_known_command` still rejects unsupported shells in direct router unit coverage.

## Step 6: Update Documentation and Help Consistency

Status: [x]

Objective:

Bring README, help text, and completion behavior into alignment.

Primary files:

- `README.md`
- `src/help.rs`
- `src/cli_spec.rs`

Implementation details:

- Verify the README `Shell Completions` section matches the implemented commands exactly:
  - `cfd completion bash > ~/.local/share/bash-completion/completions/cfd`
  - `cfd completion zsh > ~/.zfunc/_cfd`
  - `cfd completion fish > ~/.config/fish/completions/cfd.fish`
- Ensure README command list includes:
  - `cfd completion <bash|zsh|fish>`
- Ensure global help includes completion.
- Ensure `cfd help completion` and, if supported, `cfd completion help` are concise and accurate.
- Avoid adding long generated script excerpts to README.

Tests:

- Update `src/help.rs` tests if needed.
- Add or adjust CLI integration test for `cfd help completion`.

Verification:

- Run `cargo test help::`.
- Run `cargo test --test completion_cli`.

Handover:

- Summary: Verified README and help wording against the implemented `cfd completion bash|zsh|fish` behavior. Added a concise README note that completion generation writes only to stdout. Added CLI integration coverage for both `cfd help completion` and supported `cfd completion help`.
- Files changed: `README.md`, `tests/completion_cli.rs`, `PLAN.md`
- Tests run: `cargo test help::`; `cargo test --test completion_cli`
- Notes for next step: Step 7 owns formatting/full verification and final diff cleanup. Step 6 did not modify `src/help.rs` or `src/cli_spec.rs` because their existing completion wording/spec matched the final runtime behavior.

## Step 7: Full Verification and Cleanup

Status: [x]

Objective:

Run final project verification and clean up any rough edges introduced by the feature.

Primary files:

- Any files touched by earlier steps.

Implementation details:

- Run formatting.
- Run full tests.
- Inspect final diff for:
  - unrelated changes
  - overly broad abstractions
  - README/help mismatch
- Manually test completion commands:
  - `cargo run -- completion bash`
  - `cargo run -- completion zsh`
  - `cargo run -- completion fish`
  - `cargo run -- help completion`

Verification:

- Run `cargo fmt`.
- Run `cargo test`.
- Optional if time allows: `cargo clippy -- -D warnings`.

Handover:

- Summary: Ran final formatting, full verification, clippy, manual completion commands, and a focused diff review. Generated scripts are stdout-only, and manual runs succeeded for Bash, Zsh, Fish, and `help completion`.
- Files changed: `PLAN.md` for Step 7 status/handover. `cargo fmt` also left formatting-only changes in files already modified earlier in the feature branch; these were not reverted because they pre-existed this Step 7 handoff and tests pass.
- Tests run: `cargo fmt`; `cargo test` first failed in the sandbox because `tests/support/mod.rs` could not bind `127.0.0.1:0` (`PermissionDenied`), then passed with escalation for local test server binding; `cargo clippy -- -D warnings`; manual `cargo run -- completion bash`; manual `cargo run -- completion zsh`; manual `cargo run -- completion fish`; manual `cargo run -- help completion`.
- Residual risks: Completion renderers are intentionally static and broad; Bash/Zsh suggest a flattened set of known command words/options rather than context-perfect command trees. This matches the planned first implementation but could be refined later.
- Follow-up suggestions: Consider a later pass for context-aware Bash/Zsh subcommand completion if users want more precise interactive behavior.
