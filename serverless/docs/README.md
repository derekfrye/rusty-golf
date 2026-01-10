# Serverless Notes

## Deploy (Cloudflare Workers)
Prereqs:
- Install `wrangler` and authenticate with Cloudflare.
- Ensure the `wasm32-unknown-unknown` target is installed: `rustup target add wasm32-unknown-unknown`.

Deploy to dev:
```bash
wrangler deploy --config serverless/wrangler.toml --env dev
```

Deploy to prod:
```bash
wrangler deploy --config serverless/wrangler.toml --env prod
```

Notes:
- KV/R2 bindings and routes are defined per env in `serverless/wrangler.toml`.
- After deploy, seed KV/R2 for a specific event using the scripts below.

## Seed KV for an event (dev or prod)
Use `setup` to seed event details, golfers, and player_factors into the KV
namespace configured in `serverless/wrangler.toml`. You only have to seed KV; on first run the `-serverless` app will store data in R2.

Usage:
```bash
cargo run -p setup -- \
  --eup-json ~/docker/golf/eup.json \
  --kv-env dev \
  --event-id 401703521 \
  --wrangler-config serverless/wrangler.toml \
  --wrangler-env dev \
  --wrangler-flag --remote \
  --wrangler-flag --preview \
  --wrangler-flag false
```

Notes:
- `--kv-env` must be `dev` or `prod`.
- Use `--kv-binding` if you want to target a specific binding instead of the namespace id.

Verify a seeded key:
```bash
wrangler kv key get --remote --preview false \
  --namespace-id <namespace_id> \
  "event:401703521:golfers"
```

## Listing endpoint
`/listing` shows KV events when called with an auth token set via `setup`.

Example:
```
/listing?auth_token=changeme-token-1
```

If `ADMIN_ENABLED=1` and the `x-admin-token` header matches `ADMIN_TOKEN` (from `.dev.vars` in
Miniflare), `/listing` returns a JSON payload with KV and R2 keys for debugging:

```bash
curl -H "x-admin-token: $MINIFLARE_ADMIN_TOKEN" \
  "http://127.0.0.1:8787/listing"
```
