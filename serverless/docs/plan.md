# Serverless Migration Plan (Cloudflare Workers + R2)

## Goals and constraints
- Target runtime: Cloudflare Workers (Rust `wasm32-unknown-unknown`).
- Storage: R2 via S3 protocol (HTTP + AWS Signature V4).
- Preserve current UI stack: htmx + maud templates + MVU pattern.
- Remove Actix, Tokio runtime, and SQL middleware.

## Quick codebase inventory (current)
- HTTP: Actix handlers in `src/main.rs` and `src/controller/score/http_handlers.rs`.
- Views: Maud templates in `src/view/`.
- MVU: Score model/update/effects in `src/mvu/`.
- DB layer: `sql-middleware` and SQL files in `src/sql/`.
- ESPN client: `src/controller/espn/` uses `reqwest`.
- CLI args: `src/args/` for DB config and startup scripts.

## Proposed serverless structure
- Keep the existing crate for native/local use.
- Workspace layout now in place:
  - `rusty-golf-core` (shared pure logic + Storage trait)
  - `rusty-golf` (native Actix binary + SQL storage impl)
  - `serverless` (Workers entrypoint)
- Continue moving pure modules (MVU + view + DTOs) into `rusty-golf-core`.
- Keep Workers-only entrypoints and bindings in `serverless/src/lib.rs`.

## Dependency trim/replace (from `Cargo.toml`)
Remove or replace:
- `actix-web`, `actix-files`: replace with `worker` crate router + Workers static assets.
- `tokio`, `tokio-util`: avoid runtime dependencies; use `wasm-bindgen-futures` and `futures`.
- `reqwest`: replace with `worker::Fetch` (or `gloo-net` if needed).
- `deadpool-postgres`, `sql-middleware`: remove; storage moves to R2.
- `clap`: remove; replace CLI args with Workers env vars.
Keep (likely compatible):
- `serde`, `serde_json`, `maud`, `regex`, `chrono` (verify wasm support), `ahash`.
Add (likely needed):
- `worker` (workers-rs), `wasm-bindgen`, `wasm-bindgen-futures`, `aws-sign-v4` (or a minimal signer).

## Cloudflare Workers architecture mapping
- Actix `App` routes -> `worker::Router` routes.
- Actix handlers -> async functions receiving `worker::Request` and `worker::RouteContext`.
- `Data<ConfigAndPool>` -> `Env` bindings (`R2` bucket + env vars) and a shared `Storage` trait.
- Static assets: use Workers assets support with `wrangler.toml` to serve `static/`.

## Data layer redesign (SQL -> R2)
### Key idea
Replace SQL queries/stored procedures with typed JSON documents in R2, keyed by logical IDs.

### Current schema summary (SQLite)
- `event`: ESPN event metadata and refresh toggle.
- `golfer`: ESPN golfer id + name.
- `bettor`: user id + name.
- `event_user_player`: joins event + user + golfer with last refresh and step factor.
- `eup_statistic`: per event/user/golfer stats payload (rounds, scores, tee times, line scores, total score).
- `eup_statistic_hx`: history table populated by trigger on `eup_statistic` updates.

### Suggested object layout
- `events/{event_id}/event.json` -> event metadata (name, start date, etc.).
- `events/{event_id}/scores.json` -> normalized scores payload used by MVU/view.
- `events/{event_id}/golfers.json` -> golfers list and mappings.
- `cache/espn/{event_id}.json` -> raw ESPN API cache (for fallback behavior).
- `admin/last_refresh.json` -> last refresh timestamp/source.

### Proposed mapping (schema -> R2 docs)
- `event` -> `events/{event_id}/event.json` with fields `{event_id, espn_id, year, name, refresh_from_espn}`.
- `golfer` -> `events/{event_id}/golfers.json` (scoped per event to reduce fanout).
- `bettor` + `event_user_player` -> `events/{event_id}/bettors.json` with embedded picks.
- `eup_statistic` -> `events/{event_id}/scores.json` as the primary render payload.
- `eup_statistic_hx` -> optional `events/{event_id}/history/{timestamp}.json` if history is needed.

### Minimal R2-first rollout
- Start by writing `scores.json` + `event.json` directly from existing ESPN ingest code.
- Keep SQL flow unchanged for now; add a second write path to R2 to validate data shape.
- Build a small validation tool that compares SQL output vs R2 output for a sample event.


### Storage API (new trait)
Define a `Storage` trait used by controllers/models (now in `rusty-golf-core`):
- `get_event_details(event_id) -> EventDetails`
- `get_golfers_for_event(event_id) -> Vec<Scores>`
- `get_player_step_factors(event_id) -> HashMap<(i64, String), f32>`
- `get_scores(event_id, source) -> ScoresAndLastRefresh`
- `store_scores(event_id, scores) -> ()`
- `event_and_scores_already_in_db(event_id, max_age_seconds) -> bool`

Implementations (current/next):
- `SqlStorage` (done) using existing SQL functions.
- `R2Storage` using R2 bindings or S3 protocol (planned).
- Optional `InMemoryStorage` for tests.

### Concurrency and consistency
- Use ETag/If-Match for optimistic writes if required.
- Keep updates at object granularity (no multi-object transactions).
- Store derived aggregates together to reduce read fanout.

## ESPN client changes
- Replace `reqwest::Client` with `worker::Fetch` and build a small wrapper.
- Maintain offline fallback by checking R2 cache first, then remote fetch, then update cache.

## CLI/config replacement
- Move DB/CLI args to `wrangler.toml` + `Env`:
  - `ESPN_BASE_URL`, `R2_BUCKET`, `CACHE_TTL_SECONDS`.
  - Use `R2` binding name (e.g., `SCORES_BUCKET`).

## Migration phases
1. **Refactor to a storage trait** in the existing codebase; keep SQL implementation as the first backend. (done)
2. **Move pure logic** (MVU + view + DTOs) into a shared crate or module with no Actix/SQL deps. (in progress)
3. **Introduce R2 storage** and map SQL functions to JSON doc reads/writes.
4. **Create Workers entrypoint** with router and bindings; wire it to shared logic + storage.
5. **Replace ESPN client** with Workers fetch.
6. **Static assets** via Workers asset pipeline.
7. **Tests**: add storage trait unit tests + basic handler tests in Workers local dev.

## Tests and verification plan
- Unit tests for storage object mapping (serialize/deserialize, key naming).
- Worker integration tests with `wrangler dev` (manual + scripted).
- Keep existing integration tests for local build if the native binary remains.

## Open questions
- Do we need both native and worker builds long-term, or can we split into two crates?
- What R2 object schema best matches existing SQL semantics?
- Do we need per-request recomputation, or can we cache rendered HTML for hot paths?

## First concrete steps
- Add a `serverless/` crate with `worker` and `wasm32-unknown-unknown` target. (scaffolded)
- Introduce `Storage` trait in `rusty-golf-core` and refactor SQL calls behind it. (done for MVU + ESPN cache paths)
- Continue moving MVU into `rusty-golf-core`.
- Implement R2 storage and integrate into storage trait.
- Port routes to Workers router and verify `/scores` + partials render.
