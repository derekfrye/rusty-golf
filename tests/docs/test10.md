# Test 10 (serverless)

This document describes how `tests/tests/test10_serverless.rs` works and what it requires.

## Purpose
- Validates the serverless `/scores` endpoint against a local Miniflare instance.
- Seeds event data and cached ESPN responses, then checks the JSON response from `/scores`.

## Prereqs
- Miniflare running locally (see `docs/miniflare.md`).
- Environment variables set (from repo root `.env` or the shell):
  - `MINIFLARE_URL` (e.g., `http://127.0.0.1:8787`)
  - `MINIFLARE_ADMIN_TOKEN` (shared token for `/admin` endpoints)
- `wrangler` available on PATH.

## High-level flow
1. Load env vars (`MINIFLARE_URL`, `MINIFLARE_ADMIN_TOKEN`).
2. Build the local worker bundle via `serverless/scripts/build_local.sh`.
3. Wait for Miniflare health check to pass.
4. Clean up any prior event state in KV/R2 via `/admin/cleanup`.
5. Seed the event via `/admin/seed`:
   - Event details and assignments from `tests/tests/test05_dbprefill.json`.
   - Score structure and ESPN cache from `tests/tests/test03_espn_json_responses.json`.
6. Request `/scores?event=<id>&yr=2024&cache=1&json=1` and assert success.
7. Cleanup is performed at the end even on failures.

## Data inputs
- `tests/tests/test05_dbprefill.json` (event details, golfers, bettor assignments)
- `tests/tests/test03_espn_json_responses.json` (score cache used for deterministic output)

## Failure points
- Missing env vars or Miniflare not running.
- `wrangler` missing or build script failing.
- Admin token mismatch for `/admin/seed` or `/admin/cleanup`.
- Non-200 response from `/scores`.
