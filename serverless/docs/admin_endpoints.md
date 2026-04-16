# Admin And Listing Endpoints

## Auth Overview

- Admin endpoints (`/admin/*`) require `ADMIN_ENABLED=1` and `ADMIN_TOKEN`.
  - Auth is accepted via `x-admin-token` header or `admin_token` query param.
  - `/admin/cache_status` is special: it accepts either a valid `x-instrument-token`
  (matching `INSTRUMENT_TOKEN`) or a valid `auth_token` query param (same as `/listing`).
- `/listing` has a separate admin-only JSON mode gated by `x-admin-token` only.

## GET /listing

Public listing requires `auth_token` query param (validated against stored auth tokens).
Returns HTML.

Example:
```bash
curl "https://golfdev.dfrye.io/listing?auth_token=$AUTH_TOKEN"
```

Query params:
- `auth_token` (required, non-admin mode)

Admin JSON mode (only when `ADMIN_ENABLED=1` and `x-admin-token` matches `ADMIN_TOKEN`):
- `event_id` (optional, int) When provided, include `scores_exists` and
  `espn_cache_exists` for the event and omit the `r2_keys` listing.

Example (admin JSON):
```bash
curl -H "x-admin-token: $ADMIN_TOKEN" \
  "https://golfdev.dfrye.io/listing"
```

Example (admin JSON with event):
```bash
curl -H "x-admin-token: $ADMIN_TOKEN" \
  "https://golfdev.dfrye.io/listing?event_id=401580355"
```

Admin JSON response:
- `events`: array of event metadata
- `kv_keys`: list of KV keys
- `r2_keys`: list of R2 keys (empty when `event_id` is provided)
- `event_id`: echoed `event_id` or null
- `scores_exists`: bool or null
- `espn_cache_exists`: bool or null

## Admin Endpoints

All admin endpoints below require `ADMIN_ENABLED=1` plus `x-admin-token` or
`admin_token` auth. They return `"unauthorized"` (401) or `"not found"` (404) when
disabled.

### POST /admin/seed

Seeds data for an event.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data @seed_payload.json \
  "https://golfdev.dfrye.io/admin/seed"
```

JSON body (see `AdminSeedRequest` in `serverless/src/storage/storage_types.rs`):
- `event_id` (int, required)
- `refresh_from_espn` (int, required)
- `event` (object, required)
- `score_struct` (array, required)
- `espn_cache` (json, required)
- `auth_tokens` (array, optional)
- `last_refresh` (string, optional)

Response: `200 OK` with `"seeded"`.

### POST /admin/cleanup

Deletes event data (optionally auth tokens).

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355,"include_auth_tokens":false}' \
  "https://golfdev.dfrye.io/admin/cleanup"
```

JSON body:
- `event_id` (int, required)
- `include_auth_tokens` (bool, optional, default false)

Response: `200 OK` with `"cleaned"`.

### POST /admin/cleanup_scores

Deletes score data for an event.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355}' \
  "https://golfdev.dfrye.io/admin/cleanup_scores"
```

JSON body:
- `event_id` (int, required)

Response: `200 OK` with `"cleaned scores"`.

### POST /admin/cache_flush

Flushes score caches for an event.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355}' \
  "https://golfdev.dfrye.io/admin/cache_flush"
```

JSON body:
- `event_id` (int, required)

Response: `200 OK` with `"cache flushed"`.

### POST /admin/event_update_dates

Updates event start/end dates.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355,"start_date":"2024-01-10","end_date":"2024-01-14"}' \
  "https://golfdev.dfrye.io/admin/event_update_dates"
```

JSON body:
- `event_id` (int, required)
- `start_date` (string, optional)
- `end_date` (string, optional)
- `completed` (bool, optional)

Response: `200 OK` with `"updated"`.

### POST /admin/espn_fail

Forces ESPN fetch failure mode.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355,"enabled":true}' \
  "https://golfdev.dfrye.io/admin/espn_fail"
```

JSON body:
- `event_id` (int, required)
- `enabled` (bool, required)

Response: `200 OK` with `"updated"`.

### POST /admin/test_lock

Exercises the lock implementation.

Example:
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355,"token":"test-token","ttl_secs":30,"mode":"shared","force":false}' \
  "https://golfdev.dfrye.io/admin/test_lock"
```

JSON body:
- `event_id` (int, required)
- `token` (string, required)
- `ttl_secs` (int, optional; default 30)
- `mode` (string, optional; `shared` or `exclusive`, default `shared`)
- `force` (bool, optional; default false)

Response JSON:
- `acquired` (bool)
- `is_first` (bool)

### POST /admin/test_unlock

Releases a lock by event or all events.

Example (single event):
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":401580355,"token":"test-token"}' \
  "https://golfdev.dfrye.io/admin/test_unlock"
```

Example (all events):
```bash
curl -X POST -H "content-type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  --data '{"event_id":"all","token":"test-token"}' \
  "https://golfdev.dfrye.io/admin/test_unlock"
```

JSON body:
- `event_id` (int or `"all"`, required)
- `token` (string, required)

Response JSON:
- `is_last` (bool)

### GET or POST /admin/cache_status

Reports cache status for an event/year.

Example (instrument token):
```bash
curl -H "x-instrument-token: $INSTRUMENT_TOKEN" \
  "https://golfdev.dfrye.io/admin/cache_status?event=401580355&yr=2024"
```

Example (auth token):
```bash
curl "https://golfdev.dfrye.io/admin/cache_status?event=401580355&yr=2024&auth_token=$AUTH_TOKEN"
```

Query params:
- `event` (required, int)
- `yr` (required, int)
- `auth_token` (optional; required if no valid `x-instrument-token`)

Response JSON:
- `event_id`, `year`
- `in_memory`, `kv`, `r2` cache status with `exists` + `remaining_ttl_seconds`
- `keys` for KV + R2 entries
