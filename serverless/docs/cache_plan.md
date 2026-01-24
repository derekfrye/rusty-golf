# Cache Plan (KV Read-Through)

## Goal
Reduce cache-hit latency by avoiding R2 reads. KV becomes a read-through cache, while R2 remains
the source of truth. ESPN refresh logic stays the same.

## Read Path
1. Check KV first (`scores:{event_id}` or `scores:{event_id}:{year}`).
2. If KV hit, return payload immediately.
3. If KV miss, read from R2:
   - If R2 hit, return payload and write it to KV with `expiration_ttl=300`.
   - If R2 miss, fetch from ESPN, then write to R2 and KV.

## Write Path
- When storing scores (the ESPN refresh path), write to R2 and KV.
- KV is only a cache; R2 is the source of truth.

## Freshness & Worst-Case Staleness
With a KV TTL of 300s, the cache can extend the apparent age of data beyond the ESPN refresh
window. Example worst case:
- R2 last refresh is 4m59s ago when a KV miss triggers an R2 read.
- The response is written to KV with TTL 300s.
- A request just before KV expiry sees data up to ~9m59s old.

If this staleness is too high, reduce the KV TTL or embed `last_refresh` in KV and check it against
`cache_max_age` on KV hits.

## Key Notes
- KV is eventually consistent; different edges may see updates at slightly different times.
- KV value size limit: 25MB.
