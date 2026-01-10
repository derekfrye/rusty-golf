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
  - `core` (shared pure logic + Storage trait)
  - `rusty-golf` (native Actix binary + SQL storage impl)
  - `serverless` (Workers entrypoint)
- Continue moving pure modules (MVU + view + DTOs) into `core`.
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

## KV + R2 hybrid (latency/consistency balanced)
If R2 is too latent to store everything, use KV for small config + lookups and R2 for large payloads.

### Storage layout
R2 (authoritative, larger payloads):
- `events/{event_id}/scores.json` -> `ScoresAndLastRefresh` (render payload).
- `cache/espn/{event_id}.json` -> raw ESPN JSON cache.

KV (small, frequently read):
- `event:{event_id}:details` -> `{event_name, score_view_step_factor, refresh_from_espn}`.
- `event:{event_id}:golfers` -> golfer/bettor assignments (ESPN query inputs).
- `event:{event_id}:player_factors` -> `(golfer_espn_id,bettor_name) -> step_factor`.
- `event:{event_id}:last_refresh` -> `{ts, source}` for TTL checks.

### TTL-based cache_max_age flow
- `cache_max_age` derives from `event:{id}:details.refresh_from_espn`.
- On `cache=true`: read `event:{id}:last_refresh`; if fresh, read `scores.json` from R2; else fetch ESPN and overwrite.
- On `cache=false`: always fetch ESPN and overwrite.

### test1 flow mapping (cache=false)
1. `/scores` decodes request and reads `event:{id}:details` from KV for `cache_max_age`.
2. MVU `PageLoad` triggers:
   - `LoadScores`: read `event:{id}:golfers` from KV, fetch ESPN, write `cache/espn/{id}.json` and `events/{id}/scores.json` in R2, update `event:{id}:last_refresh` in KV. Return in-memory scores to avoid read-after-write.
   - `LoadEventConfig`: read `event:{id}:details` from KV.
   - `LoadPlayerFactors`: read `event:{id}:player_factors` from KV.
   - `LoadDbScores`: read `events/{id}/scores.json` from R2 (or reuse in-memory scores in this request).

### Minimal R2-first rollout
- Start by writing `scores.json` + `event.json` directly from existing ESPN ingest code.
- Keep SQL flow unchanged for now; add a second write path to R2 to validate data shape.
- Build a small validation tool that compares SQL output vs R2 output for a sample event.


### Storage API (new trait)
Define a `Storage` trait used by controllers/models (now in `core`):
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

## Production readiness gaps and plan
### Wrangler config and envs
- Add `account_id` and either `routes` or `workers_dev` to `wrangler.toml` for explicit deploy targets.
- Split config into envs (at least `dev` and `prod`) with distinct KV/R2 bindings.
- Set `preview_id` and `preview_bucket_name` so previews do not write to prod data.

### Storage bindings and configuration
- Replace hard-coded binding names with env-driven values (e.g., `KV_BINDING`, `R2_BINDING`).
- Add validation on startup to surface missing bindings with clear error messages.

### Data seeding and refresh
- Add an admin-only refresh endpoint or a CLI task to seed/refresh KV and R2 in prod.
- Document the seed flow and required KV/R2 keys for an event.

### ESPN fetch behavior
- Add request timeout and limited retry/backoff when fetching ESPN.
- Fetch player summaries concurrently with a bounded fanout to avoid worker timeouts.
- Add circuit-breaker style behavior on repeated failures (use cached ESPN data when available).

### Observability and diagnostics
- Add structured logging for fetch failures and storage misses (KV/R2 key missing).
- Add a `/health` check that validates KV and R2 bindings and returns a clear error payload.

## Proposed implementation steps
1. Update `wrangler.toml` with explicit deploy targets and per-env bindings.
2. Move binding names into env/config and refactor `ServerlessStorage::from_env` usage.
3. Add an admin refresh path or script for prod seeding and document the required data.
4. Implement ESPN fetch concurrency + timeout + retry in `ServerlessEspnClient`.
5. Add logging and health diagnostics for missing bindings and storage keys.

## First concrete steps
- Add a `serverless/` crate with `worker` and `wasm32-unknown-unknown` target. (scaffolded)
- Introduce `Storage` trait in `core` and refactor SQL calls behind it. (done for MVU + ESPN cache paths)
- Continue moving MVU into `core`.
- Implement R2 storage and integrate into storage trait.
- Port routes to Workers router and verify `/scores` + partials render.
