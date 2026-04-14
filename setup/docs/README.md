# Rusty Golf Setup

CLI utility to seed Wrangler KV entries from a db_prefill-style EUP JSON file.

## Usage

### New event mode

Start with `--mode new_event`. This mode runs an interactive REPL (unless `--one-shot` is provided). Typical usage:

```shell
cargo run -p rusty-golf-setup -- --eup-json ~/docker/golf/eup.json --kv-env prod --wrangler-config serverless/wrangler.toml --mode new_event
```

- `--kv-env prod` REPL pulls bettor names & golfer assignments from prod KV, and pre-populates REPL auto-complete from them.
- `--eup-json` used as target output if you don't pass `--output-json`. You'll be prompted for an output json if you don't specify this. 

#### Subcommands

##### `list_events`

- Fetches the current ESPN event list and caches it for the session.
- Adds any missing event IDs from the EUP JSON (if provided) by fetching names.
- If `--kv-env`/`--kv-binding` is provided, adds event IDs from KV (preferring KV names).
- Prints event IDs and names as `"<id> <name>"`.

Subcommands:

- `help`
  - Prints the `list_events` subcommand help text.
- `refresh`
  - Refreshes ESPN only, then merges those results with any cached KV events.
- `refresh espn`
  - Same as `refresh`.
- `refresh all`
  - Refreshes both ESPN and KV, then rebuilds the merged event list.
- `refresh kv`
  - Same as `kv`.
- `kv`
  - Refreshes KV only, then merges those results with any cached ESPN events.

##### `get_event_details`

- Lists events (fetching if needed), then prompts for event IDs.
- For each event, fetches the ESPN event JSON and extracts name, start date, and end date.
- Prints a table with `event_id`, `event_name`, `start_date`, and `end_date`.

##### `get_available_golfers`

- Lists events (fetching if needed), then prompts for event IDs.
- Echoes the selected event IDs as a space-separated line.
- Does not persist selections; it is a helper for copy/paste.

##### `pick_bettors`

- Loads bettor names from the EUP JSON (if available) and prompts for selection.
- Persists the selection in the session temp dir.
- Prints the selected bettors as a space-separated line.

##### `set_golfers_by_bettor`

- Prompts for golfers for each bettor, using cached event golfer data.
- Writes the selection into the REPL state (used by `setup_event`).
- Prints a JSON array per bettor with `{ bettor, golfer_espn_id }` entries.

##### `setup_event`

- Guides full event setup and writes a new EUP JSON entry.
- Reuses bettors and golfer selections if already chosen.
- Prompts for output filename and overwriting if it already exists.

##### `update_event`

- Guides event updates and writes a new EUP JSON entry.
- Always prompts for bettors, then golfers (with KV golfer assignments shown above each prompt).
- Prompts for output filename and overwriting if it already exists.

##### `exit` / `quit`

- Exits the REPL.

One-shot (`--mode new_event --one-shot`) writes JSON to `--output-json` unless `--output-json-stdout` is set.

### Mode `update_event`

This mode runs the same interactive REPL as `new_event`, but adds the
`update_event` command to show current golfer assignments from KV before
prompting for new selections.

### Seed mode

Required args (seed mode):

- `--eup-json` Path to the EUP JSON file.
- `--kv-env` Wrangler env to target (`dev` or `prod`).

Required args:

- `--mode` Operation mode (`seed`, `new_event`/`setup_event`/`repl`, `update_event`/`edit_event`, or `get_event_details`).
  - `seed`: non-interactive, reads EUP JSON and seeds KV.
  - `new_event`: interactive REPL unless `--one-shot` is provided.
  - `get_event_details`: one-shot JSON export of event details.

Optional args:

- `--config-toml` Path to a TOML config file. Values from CLI flags override it.
- `--kv-binding` KV binding name (useful for `wrangler --local`).
- `--auth-tokens` CSV list of tokens to allow `/listing?auth_token=...` access (min 8 chars each).
- `--event-id` Event id filter. For `seed` and `new_event` one-shot, this must be a single id.
  For `get_event_details`, it can be CSV or space-separated.
- `--refresh-from-espn` Value written into `event_details.json` (default: `1`).
- The setup CLI also fetches `endDate` from the ESPN scoreboard header when available and stores it in `event_details.json` to disable refreshes after the event ends.
- `--wrangler-config` Path to `wrangler.toml` (default: `serverless/wrangler.toml`).
- `--wrangler-env` Wrangler env (default: `dev`).
- `--wrangler-flag` Extra flags for all wrangler commands (repeatable).
- `--wrangler-kv-flag` Overrides `--wrangler-flag` for KV commands (repeatable).
- `--wrangler-log-dir` Directory for wrangler logs (sets `WRANGLER_LOG_DIR`).
- `--wrangler-config-dir` Directory for wrangler config (sets `XDG_CONFIG_HOME`).
- `--output-json-stdout` Write one-shot JSON output to stdout instead of a file.

### Mode `get_event_details`

This mode is non-interactive and requires `--one-shot` plus `--output-json` or
`--output-json-stdout`.

- If `--event-id` is provided, it is parsed as CSV/space-separated ids and used directly.
- If `--event-id` is omitted, it fetches the live ESPN event list and emits details for all of
  those events.
- Output is a JSON array of `{ event_id, event_name, start_date, end_date }`.
  Use `--output-json-stdout` to print the JSON instead of writing a file.

## Examples:

```bash
cargo run -q -p rusty-golf-setup -- \
  --eup-json ~/docker/golf/eup.json \
  --kv-env=dev \
  --wrangler-config=serverless/wrangler.toml \
  --auth-tokens="<token>" \
  --mode=seed \
  --wrangler-kv-flag=--preview \
  --wrangler-kv-flag=false \
  --wrangler-kv-flag=--remote
```

Note: `update_event` only reads KV for context and writes a new EUP JSON entry. It does not
seed or re-seed KV. To apply changes to KV, run `--mode seed` with the updated EUP JSON, for
example:

```bash
# 1) Update an event and write a new EUP JSON file.
cargo run -p rusty-golf-setup -- \
  --mode update_event \
  --eup-json tests/test05_dbprefill.json \
  --output-json /tmp/eup_updated.json

# 2) Re-seed KV using the updated EUP JSON.
cargo run -p rusty-golf-setup -- \
  --mode seed \
  --eup-json /tmp/eup_updated.json \
  --event-id 401703504 \
  --kv-env dev \
  --wrangler-config serverless/wrangler.toml
```

With config:

```bash
cargo run -p rusty-golf-setup -- \
  --config-toml setup/seed_config.toml \
  --event-id 401703504
```

Get event details (one-shot):

```bash
cargo run -p rusty-golf-setup -- \
  --mode get_event_details \
  --one-shot \
  --output-json /tmp/event_details.json \
  --event-id "401703504,401580355"
```

## What have I seeded?

If you do not remember which events are in KV (or whether auth tokens were set), use one of these:

- Admin listing (preferred): enable admin mode in your deployed worker (set via `.dev.vars`), then call
  `/listing` with the admin header to get a JSON response that includes KV keys and parsed event listings.
  - Set `ADMIN_ENABLED=1` and `ADMIN_TOKEN=<token>` in the worker's dev env vars.
  - Request: `curl -H "x-admin-token: <token>" "https://<your-worker>/listing"`
- Wrangler KV key list: list keys directly in the dev namespace to see seeded events. Remember to use `--remote --preview false` otherwise (for some bizarre reason) `wrangler` defaults to reading from your local env.

```shell
wrangler kv key list --env dev --binding djf_rusty_golf_kv --config serverless/wrangler.toml --remote --preview false
```
  - Seeded events show up as `event:<event_id>:details`, `event:<event_id>:golfers`, etc.

## Config file

All keys are optional. CLI values override config values.

```toml
mode = "seed"
eup_json = "tests/test05_dbprefill.json"
kv_env = "dev"
kv_binding = "djf_rusty_golf_kv"
auth_tokens = "changeme-token-1,changeme-token-2"
event_id = 401703504
refresh_from_espn = 1
wrangler_config = "serverless/wrangler.toml"
wrangler_env = "dev"
wrangler_flags = ["--config", "serverless/wrangler.toml", "--remote", "--preview", "false"]
wrangler_kv_flags = ["--config", "serverless/wrangler.toml", "--remote", "--preview", "false", "--env", "dev"]
wrangler_log_dir = "serverless/.wrangler-logs"
wrangler_config_dir = "serverless/.wrangler-config"
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
- `event:<event_id>:auth_tokens` (if `--auth-tokens` is provided)

It also writes seeded-at metadata keys:

- `event:<event_id>:details:seeded_at`
- `event:<event_id>:golfers:seeded_at`
- `event:<event_id>:player_factors:seeded_at`

When `--auth-tokens` is provided, it stores them for `/listing` access:

- `event:<event_id>:auth_tokens`



## Limitations

- KV-only: this tool does not seed R2 objects. R2 stays empty until the serverless app fetches ESPN data on a request and writes `events/<event_id>/scores.json` and `cache/espn/<event_id>.json`, or you seed them with a separate tool (note, you probably don't have to seed R2 since `-serverless` does it on first need.
