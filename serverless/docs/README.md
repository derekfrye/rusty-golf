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

Example:
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

Example:
```bash
cargo run -p rusty-golf-setup -- \
  --eup-json ~/docker/golf/eup.json \
  --kv-env prod \
  --wrangler-config serverless/wrangler.toml \
  --wrangler-env prod \
  --auth-tokens "xyz" \
  --mode seed
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
Miniflare), `/listing` further returns a JSON payload with KV and R2 keys for debugging:

```bash
curl -H "x-admin-token: $MINIFLARE_ADMIN_TOKEN" \
  "http://127.0.0.1:8787/listing"
```

## Instrumentation (per-request timings)
Serverless timings are opt-in and gated by the `INSTRUMENT_TOKEN` secret. When enabled, a single
JSON log line is emitted per request with total time and phase breakdowns (cache, ESPN, storage,
render, response).

Set the secret (per env):
```bash
wrangler secret put INSTRUMENT_TOKEN --env dev
```

Enable for a request:
```bash
curl -H "x-instrument-token: $INSTRUMENT_TOKEN" \
  "https://your-worker.example.com/scores?event=401703521&yr=2025"
```

Logs show up under Workers Observability → Logs, or via:
```bash
wrangler tail --format json --env dev
```

Parsing notes (the tail file is a stream of JSON objects, each may contain `logs[].message[]` with
stringified JSON from `console_log`):

See `serverless/docs/Timing_results.md` for a saved analysis and parsing snippet.

Python: scan for `type == "instrumentation"` and compute max phase/total.
```bash
python - <<'PY'
import json
from pathlib import Path

text = Path("worker-tail.jsonl").read_text()
idx = 0
max_phase = (None, -1.0, None)  # (payload, ms, name)
max_total = (None, -1.0)
count = 0

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
                if isinstance(ms, (int, float)) and ms > max_phase[1]:
                    max_phase = (payload, ms, name)

print("instrumentation entries:", count)
print("max phase:", max_phase[2], max_phase[1])
print("max total:", max_total[1])
PY
```

jq: extract phase timings (helpful for quick greps).
```bash
jq -r '
  select(.logs)
  | .logs[]
  | .message[]
  | fromjson?
  | select(.type=="instrumentation")
  | .phases[]
  | "\(.name)\t\(.ms)"
' worker-tail.jsonl
```

## Offline HTML validation (v.Nu)
Use the Nu Html Checker container to validate rendered HTML without the online UI.

Validate a public URL:
```bash
podman run --rm ghcr.io/validator/validator:latest \
  vnu --format json --stdout "https://example.com" > vnu-report.json
```

Validate a local dev URL:
```bash
podman run --rm --network host ghcr.io/validator/validator:latest \
  vnu --format json --stdout "http://localhost:8787" > vnu-report.json
```

Notes:
- The `--network host` flag is required if the page is only reachable on localhost.
- Use `--format text` instead of `json` if you want human-readable output.
