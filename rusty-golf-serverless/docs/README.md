# Serverless Notes

## Deploy (Cloudflare Workers)
Prereqs:
- Install `wrangler` and authenticate with Cloudflare.
- Ensure the `wasm32-unknown-unknown` target is installed: `rustup target add wasm32-unknown-unknown`.

Deploy to dev:
```bash
wrangler deploy --config rusty-golf-serverless/wrangler.toml --env dev
```

Deploy to prod:
```bash
wrangler deploy --config rusty-golf-serverless/wrangler.toml --env prod
```

Notes:
- KV/R2 bindings and routes are defined per env in `rusty-golf-serverless/wrangler.toml`.
- After deploy, seed KV/R2 for a specific event using the scripts below.

## Seed KV for an event (dev or prod)
Use `rusty-golf-setup` to seed event details, golfers, and player_factors into the KV
namespace configured in `rusty-golf-serverless/wrangler.toml`. You only have to seed KV; on first run the `-serverless` app will store data in R2.

Usage:
```bash
cargo run -p rusty-golf-setup -- \
  --eup-json ~/docker/golf/eup.json \
  --kv-env dev \
  --event-id 401703521 \
  --wrangler-config rusty-golf-serverless/wrangler.toml \
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
