# Repository Guidelines

## Project Structure & Module Organization
- Root: `README.md`, `LICENSE`, `AGENTS.md` (this file).
- Source lives under `src/` (by domain), tests under `tests/` mirroring `src/`.
- Support folders: `scripts/` (dev helpers), `docs/` (design notes), `assets/` (static files), `ci/` (automation configs), `tools/` (linters/hooks).
- Example layout:
  - `src/core/`, `src/cli/`, `tests/core/`, `tests/cli/`.

## Build, Test, and Development Commands
- Prefer Make targets when present:
  - `make setup`: install toolchain/deps.
  - `make build`: compile or bundle sources.
  - `make test`: run unit/integration tests with coverage.
  - `make lint`: run format and lint checks.
  - `make run`: start local app/CLI.
- If no `Makefile`, use scripts under `scripts/` (e.g., `scripts/dev.sh`, `scripts/test.sh`). Keep commands reproducible and non‑interactive.

## Coding Style & Naming Conventions
- Indentation: 2 spaces (unless language tooling enforces otherwise).
- Line length: 100 chars target; wrap thoughtfully for readability.
- Naming: directories `kebab-case/`, files `snake_case.ext`, types/classes `PascalCase`, variables `camelCase`.
- Formatting/Linting: adopt the language’s standard (e.g., `prettier`, `eslint`, `black`, `ruff`, `gofmt`). Place configs in repo and wire to `make lint`.

## Testing Guidelines
- Place tests in `tests/` mirroring `src/` structure.
- Naming: `tests/<module>_test.ext` or `src/**/__tests__/*.test.ext` depending on ecosystem.
- Coverage: aim ≥ 80% for changed code; include edge cases and failure paths.
- Run locally with `make test` (or `scripts/test.sh`) before opening a PR.

## Commit & Pull Request Guidelines
- Commits: follow Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`). Keep commits small and focused.
- Branches: `feature/<short-slug>`, `fix/<issue-id>-<slug>`.
- PRs: clear description, linked issues (e.g., `Closes #123`), rationale, test evidence (logs/screenshots), and notes on breaking changes or migrations.
- CI must pass; include docs updates when behavior changes.

## Security & Configuration Tips
- Never commit secrets. Use `.env` locally and provide `.env.example` with safe defaults.
- Add ignores to `.gitignore`. Review licenses for new dependencies.
- Prefer least-privilege credentials and rotate tokens used in CI.
