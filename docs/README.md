# Rusty golf

Rusty golf is a multi-crate Rust workspace for a single-page golf tournament scoreboard, pulling stats from ESPN's live PGA API. It can run in the free tier of Cloudflare's excellent Workers platform, or you can host it the traditional way via actix and store all state in a database (sqlite or postgres).

Shared logic lives in `core/`, with runtime-specific servers in `actix/` (Actix Web) and `serverless/` (Cloudflare Workers). If you use the `actix/` flavor, data lives in Postgres or SQLite (examples below assume SQLite). If you use the `serverless/` flavor, data lives in KV/R2. For `serverless/`, there's also a `setup/` CLI helps to seed events and KV. 

## Getting started (Serverless flavor)

Here's instructions for running on your host as-is, which requires `wrangler` to be installed. If you'd prefer setting up in a container, look at [Miniflare instructions](/docs/miniflare.md).

1. Install `wrangler`.
2. Install the WASM target: `rustup target add wasm32-unknown-unknown`.
3. For local deployment, `serverless/wrangler.toml` should work fine as-is. For real, remote deploys, authenticate with Cloudflare and update `serverless/wrangler.toml` with your `account_id`, routes, KV namespace IDs, and R2 bucket names. 


Local-only dev (no Cloudflare deploy):
```bash
wrangler dev --local --config serverless/wrangler.toml --env dev
```

Seed local KV (for `wrangler dev --local`):

```bash
cargo run -p rusty-golf-setup -- \
  --eup-json tests/tests/test05_dbprefill.json \
  --kv-env dev \
  --event-id 401703521 \
  --wrangler-config serverless/wrangler.toml \
  --wrangler-env dev \
  --kv-binding djf_rusty_golf_kv \
  --wrangler-kv-flag --local
```

Browse to the site:
```
http://127.0.0.1:8787/?event=401703521&yr=2025
```

## Getting started (Actix flavor)

Actix runs as a standard web server and persists data to SQLite or Postgres.

```shell
cargo run -p rusty-golf-actix -- \
  --db-type=sqlite \
  --db-name=rusty_golf.db \
  --db-startup-script=examples/init_db.sql \
  --db-populate-json=tests/tests/test05_dbprefill.json
```

Now you're ready to visit the site.

```shell
python -m webbrowser http://127.0.0.1:5201/?event=401580351&yr=2024
```

## Repository layout
- `actix/`: Actix web server crate (runtime-specific wiring)
- `core/`: Shared domain/model/storage logic
- `serverless/`: Cloudflare Workers crate (runtime-specific wiring)
- `setup/`: CLI for bootstrapping events and seeding KV
- `tests/`: Integration test crate and fixtures
- `static/`: Static assets (js, css, etc.) served at `/static`
- `docs/`: Project documentation

## Adding a tournament

Future versions may automate this process.

1. Go [here](https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn), get the event ID.
2. Go here https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=&lt;eventId&gt;, find all the golfers you want to include.
3. Populate [db_prefill.json](tests/tests/test05_dbprefill.json) with the data you need for your tournament.
4. Restart with podman-compose; if using the [example docker-compose.yml](examples/docker-compose.yml), it'll read the db_prefill.json and load the data into the sqlite database.

## Postgresql Debugging (Actix flavor)
If you want to use Postgres with the `actix/` server and debug locally, create a `.env` file in the *root* of this project[^1]. VScode debugging has been tested. Your `.env` will be excluded by `.dockerignore`. Specify your Postgres `<values>` below based on your needs. If you're using the example podman compose file, make sure your DB_PORT and DB_HOST match entries in `examples/docker-compose.yml`.
```text
DB_USER=<string>
DB_PASSWORD=<string>
DB_HOST=<ip4 is tested, but ipv6 might work, idk>
DB_NAME=<string>
DB_PORT=<integers>
```

## Testing
- Recommended: `cargo nextest run --no-fail-fast`
- Fallback: `cargo test`
- Most tests use in-memory SQLite; no Postgres required.
- Serverless tests (`test10_serverless.rs`, `test11_listing.rs`) require a running Miniflare instance and admin token in `.env` file of repo root[^1].
- Offline mode: when ESPN HTTP requests fail (e.g., no network), tests automatically fall back to local fixtures so the suite remains deterministic. See [testing documentation](tests.md).

## How we use `htmx`

See [htmx documentation](htmx.md) for how we use `htmx`.

[^1]: Where's the *root* of the project? The root of the project is alongside the `LICENSE` file. Create a `.env` file there for debugging with VScode and for tests.
    <pre>
    .
    ├── Cargo.toml
    ├── Dockerfile
    ├── docs
    │   └── README.md
    <b>├── .env</b>
    ├── examples
    │   ├── docker-compose.yml
    │   ├── Dockerfile -> ../Dockerfile
    │   └── init_db.sql
    ├── LICENSE
    ├── actix
    ├── core
    ├── serverless
    ├── setup
    ├── tests
    ...
    </pre>
