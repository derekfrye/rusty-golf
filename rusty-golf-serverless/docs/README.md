# Serverless Notes

## Seed KV for an event (dev or prod)
The seed script writes event details, golfers, and player_factors into the KV namespace
configured in `rusty-golf-serverless/wrangler.toml` for the selected env.

Usage:
```bash
rusty-golf-serverless/scripts/seed_kv_from_eup.sh \
  ~/docker/golf/eup.json dev 401703521
```

Notes:
- The second argument must be `dev` or `prod`.
- The script looks up `kv_namespaces[0].id` for that env; it fails if missing.
- Wrangler runs in remote mode by default in the script.

Verify a seeded key:
```bash
wrangler kv key get --remote --preview false \
  --namespace-id <namespace_id> \
  "event:401703521:golfers"
```
