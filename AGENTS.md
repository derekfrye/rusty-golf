# Repository Guidelines

## Project Structure & Module Organization
- `src/` – Rust sources organized by feature:
  - `controller/` (HTTP handlers, data services), `model/` (DB + domain types), `view/` (templates/components), `args/` (CLI args), `sql/` (schemas).
- `tests/` – Integration tests with fixtures (JSON/SQL, debug HTML).
- `static/` – Public assets served at `/static`.
- `docs/` – Project docs (build, tests, htmx usage).
- `examples/` – `docker-compose.yml` and init SQL for local runs.

## Build, Test, and Development Commands
- Build: `cargo build`
- Lint: `cargo clippy -D warnings`
- Format: `cargo fmt`
- Run (SQLite example):
  - `cargo run -- --db-type=sqlite --db-name=/tmp/rusty_golf.db --db-startup-script=examples/init_db.sql --db-populate-json=tests/test5_dbprefill.json`
- Tests (preferred): `cargo nextest run --no-fail-fast` (see `docs/tests.md`)
- Container image: `make build` / `make clean` / `make rebuild` (uses Podman)

## Coding Style & Naming Conventions
- Rust edition 2024; use `cargo fmt` (4‑space indent, rustfmt defaults).
- Names: modules/files `snake_case`, types/traits `UpperCamelCase`, functions/vars `snake_case`.
- Keep modules small and feature‑oriented under `controller/`, `model/`, `view/`.
- Prefer `Result<T, E>` with `?`; avoid panics in library code.
- Run `cargo clippy -D warnings` before pushing.

## Testing Guidelines
- Integration tests live in `tests/` (e.g., `test1_test_scores.rs`, `test4_cache.rs`).
- Use Nextest: `cargo nextest run --no-fail-fast`; fallback: `cargo test`.
- Offline fallback: if ESPN HTTP fails, tests auto-load `tests/test3_espn_json_responses.json` and persist via normal DB path for deterministic runs.
- Tests use in‑memory SQLite and load schemas from `src/sql/schema/*`.
- When adding features, include a focused integration test and any fixtures under `tests/`.

## Commit & Pull Request Guidelines
- Commits: short, imperative subject (e.g., "fix tests", "add cache check"). Group related changes.
- PRs: include a clear description, linked issues, test coverage notes, and screenshots of `/scores` when UI is affected.
- CI hygiene: ensure `cargo build`, `cargo test`, `cargo fmt`, and `cargo clippy -D warnings` all pass.

## Security & Configuration Tips
- Do not commit secrets. For Postgres, mount a secret file (e.g., `/secrets/db_password`) or use env vars. For local debug, create a root‑level `.env` as described in `docs/README.md`.
- Prefer SQLite for quick local runs; use `examples/docker-compose.yml` to mirror production settings.

## Architecture Notes (MVU)
- Server-side MVU for scores: see `src/mvu/score.rs`.
- Overview and guidance: `docs/mvu.md`.
- Update is pure (Model + Msg -> Effects); IO runs in an async effects runner; views are pure (`render_scores_template_pure`).
- Request-driven only (no background tick/hx-trigger); htmx remains for partials.
- Start new UI flows with the same pattern: define Model/Msg/Effect, keep rendering functions IO-free, and compose via the handler.
