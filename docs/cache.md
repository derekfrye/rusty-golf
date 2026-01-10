# Cache behavior

This document describes how score caching works and how it differs between the Actix and Serverless runtimes.

## Shared rules
- Requests default to cached mode. Use `cache=0` in the query string to force a fresh ESPN fetch.
- Cache max age is derived from event details:
  - `refresh_from_espn = 1` -> 300 seconds.
  - Any other value -> 0 (treat as no cache).
- If `end_date` is present and in the past, the cache is treated as effectively permanent:
  - When cached scores exist, they are returned and ESPN is not called.
  - If no cached scores exist, a fetch still occurs to seed the cache.
- If ESPN fetch fails, the system falls back to cached scores (if any).

## Serverless behavior
- Cached scores live in R2 at `events/<event_id>/scores.json`.
- If `scores.json` is missing or unreadable and ESPN cannot be reached, the serverless runtime falls back to the seeded ESPN cache at `cache/espn/<event_id>.json`.
- Last refresh metadata is stored in KV (`event:<event_id>:last_refresh`).
- Freshness check compares the KV timestamp to the cache max age in seconds.
- With `cache=0`, ESPN is polled on every request (fallback to cached if ESPN fails).

## Actix behavior
- Cached scores live in the SQL database (`eup_statistic` and related tables).
- Freshness check uses `min(ins_ts)` from `eup_statistic` and compares `diff.num_days()` against `cache_max_age`.
  - Because `cache_max_age` is expressed in seconds (300 by default), this effectively treats cached data as valid for about 300 days.
  - This is the current behavior and means Actix re-polls ESPN much less frequently once data exists.
- With `cache=0`, ESPN is polled on every request (fallback to cached if ESPN fails).

## Notes
- `end_date` is populated by the `setup` seed process when it can fetch the ESPN scoreboard header; it is stored in event details and used by both runtimes.
