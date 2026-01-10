# Serverless Test Locks

This document explains how serverless tests coordinate shared KV/R2 state and how to force clear locks.

## Overview
- Tests can run in parallel with `nextest`, so serverless tests need coordination to avoid clobbering KV/R2 state.
- The worker exposes `/admin/test_lock` and `/admin/test_unlock` endpoints backed by KV.
- Locks are per event id and support **shared** and **exclusive** modes.

## Modes
- **shared**: multiple tests may hold a shared lock for the same event id concurrently.
- **exclusive**: only one test may hold an exclusive lock for an event id; it blocks all shared holders too.

## Behavior
- The first holder (shared or exclusive) is responsible for seed/cleanup setup.
- The last holder to release is responsible for cleanup teardown.
- Locks are leased with a TTL (30 seconds) and expired holders are pruned on every lock/unlock.
- Tests retry lock acquisition every 250ms, timing out after 10 seconds.

## Force clear
If a test crashes and leaves a stale lock, you can force clear it using `force=true` on `/admin/test_lock`.

Example (exclusive, force clear):

```bash
curl -i -X POST "$MINIFLARE_URL/admin/test_lock" \
  -H "x-admin-token: $MINIFLARE_ADMIN_TOKEN" \
  -H "content-type: application/json" \
  -d '{"event_id":401580351,"token":"manual-clear","ttl_secs":30,"mode":"exclusive","force":true}'
```

If you want to release the lock manually:

```bash
curl -i -X POST "$MINIFLARE_URL/admin/test_unlock" \
  -H "x-admin-token: $MINIFLARE_ADMIN_TOKEN" \
  -H "content-type: application/json" \
  -d '{"event_id":401580351,"token":"manual-clear"}'
```

To clear all test locks:

```bash
curl -i -X POST "$MINIFLARE_URL/admin/test_unlock" \
  -H "x-admin-token: $MINIFLARE_ADMIN_TOKEN" \
  -H "content-type: application/json" \
  -d '{"event_id":"all","token":"manual-clear"}'
```
