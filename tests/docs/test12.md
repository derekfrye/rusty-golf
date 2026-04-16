# Test 12 (serverless cache)

This document describes how `tests/tests/test12_srvless_cache.rs` works and what it requires.

## Purpose
- Validates serverless `/scores` cache behavior based on `completed`.
- Uses cached ESPN header fixture to avoid hitting ESPN directly.
- Verifies that `end_date` alone no longer freezes refreshes, and that `completed=true` does.

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
6. Load `endDate` from `tests/tests/test12_espn_header.json` and normalize it to the past.
7. Update the KV event details via `/admin/event_update_dates` with `completed=false`.
8. Fetch `/scores?event=<id>&yr=2024&json=1` and assert `cache_hit=false`.
9. Re-seed the event with `completed=true`.
10. Fetch `/scores?event=<id>&yr=2024&json=1` and assert `cache_hit=true`.
11. Cleanup is performed at the end even on failures.

## Data inputs
- `tests/tests/test05_dbprefill.json` (event details, golfers, bettor assignments)
- `tests/tests/test03_espn_json_responses.json` (score cache used for deterministic output)
- `tests/tests/test12_espn_header.json` (ESPN header fixture for end dates and completion state)

## Failure points
- Missing env vars or Miniflare not running.
- `wrangler` missing or build script failing.
- Admin token mismatch for `/admin` endpoints.
- `/admin/event_update_dates` missing (requires updated worker bundle).
- Non-200 response from `/scores` or missing `cache_hit` field in JSON.
