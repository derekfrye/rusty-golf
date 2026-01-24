# Timing Results

Captured with `x-instrument-token` enabled (4 instrumented requests).

Highlights:
- Max total request time: ~2606ms
- Max single phase: `score_context.load_ms` ~2499ms

Request breakdown (4 instrumented requests):
- Event 401580360 (slowest, total ~2606ms): dominant phases were `score_context.load_ms` (~2499ms),
  `score_context.fetch_scores_ms` (~2216ms), `storage.store_scores_ms` (~1136ms),
  `espn.fetch_json_ms`/`espn.fetch_total_ms` (~745ms), and `storage.r2_put_json_ms` (~609ms).
- Event 401703521 (total ~556ms): dominant phases were `score_context.load_ms` (~442ms),
  `storage.r2_get_json_fetch_ms`/`cache.get_scores_db_ms`/`score_context.fetch_scores_ms` (~187ms).
- Event 401580351 (total ~538ms): dominant phases were `score_context.load_ms` (~428ms),
  `storage.r2_get_json_fetch_ms`/`cache.get_scores_db_ms`/`score_context.fetch_scores_ms` (~219ms).
- Event 401580355 (total ~570ms): dominant phases were `score_context.load_ms` (~455ms),
  `storage.r2_get_json_fetch_ms`/`cache.get_scores_db_ms`/`score_context.fetch_scores_ms` (~202ms).

Takeaway: The slowest request is dominated by ESPN fetch + R2 store. The other three are cache hits
dominated by the R2 fetch path.

## Summary Table

| Event ID | Total (ms) | Dominant phases (ms) |
| --- | --- | --- |
| 401580360 | ~2606 | `score_context.load_ms` (~2499), `score_context.fetch_scores_ms` (~2216), `storage.store_scores_ms` (~1136), `espn.fetch_json_ms` (~745), `storage.r2_put_json_ms` (~609) |
| 401703521 | ~556 | `score_context.load_ms` (~442), `storage.r2_get_json_fetch_ms` (~187), `cache.get_scores_db_ms` (~187), `score_context.fetch_scores_ms` (~187) |
| 401580351 | ~538 | `score_context.load_ms` (~428), `storage.r2_get_json_fetch_ms` (~219), `cache.get_scores_db_ms` (~219), `score_context.fetch_scores_ms` (~219) |
| 401580355 | ~570 | `score_context.load_ms` (~455), `storage.r2_get_json_fetch_ms` (~202), `cache.get_scores_db_ms` (~202), `score_context.fetch_scores_ms` (~202) |

## Optimization

### Cache-hit requests

The cache-hit path is dominated by `storage.r2_get_json_fetch_ms` (~200ms). The best wins are to
avoid R2 reads or shrink the payload:

- Edge cache via `caches.default`: check cache before R2, `cache.put()` JSON on R2 hit with a short
  TTL. This can avoid R2 entirely for bursts of traffic and keep per-request CPU low. Use a cache
  key that includes `event_id` and, if needed, a cache-busting version (for example, last refresh
  timestamp or a `cache_max_age` window).
- Per-worker in-memory cache: keep the last `ScoresAndLastRefresh` in a global cache with a short
  TTL (5–30s). This avoids R2 on a warm worker instance and is the cheapest change. It will not
  help cold starts, but it smooths out spikes.
- KV instead of R2: KV reads are often faster than R2, but confirm payload size is within KV limits
  (25MB). KV is eventually consistent, so decide if you are ok with brief staleness after refresh.
  If you keep R2 as the source of truth, you can still populate KV as a read-through cache.
- Reduce payload size: if `scores.json` is large, shrink what you store (strip unused fields, store
  a view-model payload instead of raw ESPN data) to reduce R2 fetch time and JSON parse time.
- Optional: store a compressed payload (gz) and decode in the Worker. This trades a small CPU cost
  for lower transfer time; only worth it if payloads are large and CPU headroom exists.

## Cache Status Endpoint

Admin endpoint to inspect cache presence/TTLs for a given event/year. Authorized if either the
`x-instrument-token` header is valid or a seeded `auth_token` is provided.

Example usage:

```bash
curl -H "x-instrument-token: $TOKEN" \
  "https://golfdev.dfrye.io/admin/cache_status?event=401703515&yr=2025"
```

```bash
curl "https://golfdev.dfrye.io/admin/cache_status?event=401703515&yr=2025&auth_token=$AUTH_TOKEN"
```

Response shape:

```json
{
  "event_id": 401703515,
  "year": 2025,
  "in_memory": { "exists": true, "remaining_ttl_seconds": 18 },
  "kv": { "exists": true, "remaining_ttl_seconds": 244 },
  "r2": { "exists": true, "remaining_ttl_seconds": null },
  "keys": { "kv": "event:401703515:scores_cache", "r2_scores": "events/401703515/scores.json" }
}
```

## Parsing Script

```bash
python - <<'PY'
import json
from pathlib import Path

text = Path("worker-tail.jsonl").read_text()
idx = 0
count = 0
max_phase = (None, -1.0, None)  # (payload, ms, name)
max_total = (None, -1.0)
phase_totals = {}
phase_counts = {}

while idx < len(text):
    while idx < len(text) and text[idx].isspace():
        idx += 1
    if idx >= len(text):
        break
    obj, next_idx = json.JSONDecoder().raw_decode(text, idx)
    idx = next_idx
    for entry in obj.get("logs", []):
        for msg in entry.get("message", []):
            try:
                payload = json.loads(msg)
            except json.JSONDecodeError:
                continue
            if payload.get("type") != "instrumentation":
                continue
            count += 1
            total = payload.get("total_ms")
            if isinstance(total, (int, float)) and total > max_total[1]:
                max_total = (payload, total)
            for phase in payload.get("phases", []):
                ms = phase.get("ms")
                name = phase.get("name")
                if isinstance(ms, (int, float)):
                    phase_totals[name] = phase_totals.get(name, 0.0) + ms
                    phase_counts[name] = phase_counts.get(name, 0) + 1
                    if ms > max_phase[1]:
                        max_phase = (payload, ms, name)

print("instrumentation entries:", count)
print("max phase:", max_phase[2], max_phase[1])
print("max total:", max_total[1])
if phase_totals:
    avg = sorted(
        ((name, phase_totals[name] / phase_counts[name]) for name in phase_totals),
        key=lambda x: x[1],
        reverse=True,
    )
    print("top avg phases:")
    for name, ms in avg[:5]:
        print(f"  {name}: {ms:.1f}ms")
PY
```
