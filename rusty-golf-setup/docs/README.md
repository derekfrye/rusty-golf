# Rusty Golf Setup

CLI utility to seed Wrangler KV entries from a db_prefill-style EUP JSON file.

## Usage

Required args:

- `--eup-json` Path to the EUP JSON file.
- `--kv-env` Wrangler env to target (`dev` or `prod`).

Optional args:

- `--config-toml` Path to a TOML config file. Values from CLI flags override it.
- `--kv-binding` KV binding name (useful for `wrangler --local`).
- `--event-id` Filter to a single event id.
- `--refresh-from-espn` Value written into `event_details.json` (default: `1`).
- `--wrangler-config` Path to `wrangler.toml` (default: `rusty-golf-serverless/wrangler.toml`).
- `--wrangler-env` Wrangler env (default: `dev`).
- `--wrangler-flag` Extra flags for all wrangler commands (repeatable).
- `--wrangler-kv-flag` Overrides `--wrangler-flag` for KV commands (repeatable).
- `--wrangler-log-dir` Directory for wrangler logs (sets `WRANGLER_LOG_DIR`).
- `--wrangler-config-dir` Directory for wrangler config (sets `XDG_CONFIG_HOME`).

Example:

```bash
cargo run -p rusty-golf-setup -- \
  --eup-json tests/test5_dbprefill.json \
  --kv-env dev
```

With config:

```bash
cargo run -p rusty-golf-setup -- \
  --config-toml rusty-golf-setup/seed_config.toml \
  --event-id 401703504
```

## Config file

All keys are optional. CLI values override config values.

```toml
eup_json = "tests/test5_dbprefill.json"
kv_env = "dev"
kv_binding = "djf_rusty_golf_kv"
event_id = 401703504
refresh_from_espn = 1
wrangler_config = "rusty-golf-serverless/wrangler.toml"
wrangler_env = "dev"
wrangler_flags = ["--config", "rusty-golf-serverless/wrangler.toml", "--remote", "--preview", "false"]
wrangler_kv_flags = ["--config", "rusty-golf-serverless/wrangler.toml", "--remote", "--preview", "false", "--env", "dev"]
wrangler_log_dir = "rusty-golf-serverless/.wrangler-logs"
wrangler_config_dir = "rusty-golf-serverless/.wrangler-config"
```

## Output

For each event, the tool writes:

- `event_details.json`
- `golfers.json`
- `player_factors.json`

Then it uploads them to KV using:

- `event:<event_id>:details`
- `event:<event_id>:golfers`
- `event:<event_id>:player_factors`
