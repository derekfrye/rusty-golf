# Cache behavior

This document describes how score caching works and how it differs between the Actix and Serverless runtimes.

## Shared rules
- Requests default to cached mode. Use `cache=0` in the query string to force a fresh ESPN fetch.
- Cache max age is derived from event details:
  - `refresh_from_espn = 1` -> 300 seconds.
  - Any other value -> 0 (treat as no cache).
- If `completed = true`, the cache is treated as effectively permanent:
  - When cached scores exist, they are returned and ESPN is not called.
  - If no cached scores exist, a fetch still occurs to seed the cache.
- If ESPN fetch fails, the system falls back to cached scores (if any).
- In serverless, `completed = true` can also be promoted on a successful ESPN refresh when the
  ESPN scoreboard header reports completion and the event `end_date` is more than 5 days in the past.
  The promotion check runs after `store_scores()`, not on cache hits.

## Serverless behavior
- Cached scores live in R2 at `events/<event_id>/scores.json`.
- If `scores.json` is missing or unreadable and ESPN cannot be reached, the serverless runtime falls back to the seeded ESPN cache at `cache/espn/<event_id>.json`.
- Last refresh metadata is stored in KV (`event:<event_id>:last_refresh`).
- Freshness check compares the KV timestamp to the cache max age in seconds.
- With `cache=0`, ESPN is polled on every request (fallback to cached if ESPN fails).
- Completion promotion reads ESPN `scoreboard/header` for `sport=golf&league=pga` and treats either `fullStatus.completed` or `fullStatus.type.completed` as completed.
- If the stored event details already have `completed = true`, the serverless promotion path returns immediately.
- If `end_date` is missing or not yet older than 5 days, the serverless promotion path returns without changing `completed`.

## Actix behavior
- Cached scores live in the SQL database (`eup_statistic` and related tables).
- Freshness check uses `min(ins_ts)` from `eup_statistic` and compares `diff.num_days()` against `cache_max_age`.
  - Because `cache_max_age` is expressed in seconds (300 by default), this effectively treats cached data as valid for about 300 days.
  - This is the current behavior and means Actix re-polls ESPN much less frequently once data exists.
- With `cache=0`, ESPN is polled on every request (fallback to cached if ESPN fails).

## Notes
- `completed` is initially populated by the `setup` seed process from ESPN scoreboard/event completion state.
