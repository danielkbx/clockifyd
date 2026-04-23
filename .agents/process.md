# Development Process

## Planning

Before implementing any non-trivial task, read all files in `.agents/` and include that step explicitly in the plan.

## Tooling

| Task | Command |
|---|---|
| Dev run | `cargo run -- <args>` |
| Build release | `cargo build --release` |
| Test | `cargo test` |
| Lint | `cargo clippy -- -D warnings` |
| Format | `cargo fmt` |
| Size check | `ls -lh target/release/cfd` |

## Commits

- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`
- Scope optional: `feat(entry): add overlap warning`
- Message describes why, not what
- Stage specific files, never `git add .`
- Never commit `.env`, API keys, or `target/`

## Branching

- `main` — stable, releasable
- `feat/<name>` — new features
- `fix/<name>` — bug fixes

## Config Files

- `~/.config/cfd/config.json` — user config
- `.env` — local dev overrides, never committed

## Environment Variables

| Variable | Purpose |
|---|---|
| `CFD_CONFIG` | Custom config file path |
| `CFD_WORKSPACE` | Default workspace override |
| `CFD_ROUNDING` | Default rounding override |
| `CLOCKIFY_API_KEY` | Override config API key |
| `CFD_BASE_URL` | Test-only base URL override for local mock server |
| `CLOCKIFY_TEST_API_KEY` | Integration test API key |
| `CLOCKIFY_TEST_WORKSPACE` | Integration test workspace ID |
| `CLOCKIFY_TEST_PROJECT` | Integration test project ID |

## Safety Rules

- API keys must never appear in stdout, stderr, logs, or fixtures
- Output should stay compact for agent use
- Do not add dependencies without a concrete need
- Prefer stable request shapes directly backed by the Clockify docs
