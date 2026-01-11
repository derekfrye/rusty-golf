# Miniflare (rootless Podman)

This guide uses `examples/miniflare/Makefile` to build a local rootless Podman Miniflare container, which is useful to run the serverless tests for this project.

## Prereqs
- Rootless Podman.
- `wrangler` and `rustup target add wasm32-unknown-unknown` installed on your host (tests run `wrangler build` locally).

## Build the image and seed the volume
From the repo root, this: 
1. Creates a podman volume named `miniflare_working`.
2. Uses an ephemeral alpine container to copy `core/`, `serverless/`, and `static/` into the `miniflare_working` volume, along with a script to work around a `worker-build` bug.
3. Builds the Miniflare image as `localhost/djf/m-golf-srvless`.
```bash
make -C examples/miniflare
```

## Create admin token so Miniflare image accepts KV/R2 data seeding
Under the user account that will run miniflare container, generate a token and store it as a Podman secret. We create this because miniflare container requires this token in `.dev.vars` of the container, and checks its value against all requests to `/admin` pages. This pre-shared token restricts access to the list, seed, and delete functionality for KV/R2 data in the container. 

```bash
openssl rand -base64 24 | tr -dc '[:print:]' | tr -d '[:space:]' | head -c 16 > miniflare_token.txt
podman secret create miniflare_token miniflare_token.txt
```
Keep the token value for the `.env` setup below, we'll delete it after.

## Start Miniflare
To start the miniflare container:

```bash
podman run --rm --name miniflare \
  -p 8787:8787 \
  -v miniflare_working:/work \
  --secret miniflare_token,type=mount,target=/run/secrets/miniflare_token,mode=0400 \
  -e WRANGLER_LOG_DIR=/tmp/wrangler-logs \
  -e XDG_CONFIG_HOME=/tmp/wrangler-config \
  -e WRANGLER_BUILD_LOCK_DIR=/tmp/wrangler-locks \
  -e WRANGLER_ENV=dev \
  -e RUSTFLAGS='--cfg=getrandom_backend="wasm_js"' \
  -e WORKER_BUILD_BIN=/work/bin/worker-build \
  -e PATH=/work/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  -w /work \
  localhost/djf/m-golf-srvless:latest \
  sh -c 'token="$(cat /run/secrets/miniflare_token)"; echo "ADMIN_ENABLED=1" > /work/serverless/.dev.vars; echo "ADMIN_TOKEN=$token" >> /work/serverless/.dev.vars; exec wrangler dev --local --port 8787 --ip 0.0.0.0 --config /work/serverless/wrangler.toml --env dev --persist-to /work/.wrangler/state'
```

Miniflare is now running on `http://127.0.0.1:8787`.

Alternatively, to make this a more permanent running container using a quadlet, copy the quadlet file into your user systemd config and start it:
```bash
mkdir -p ~/.config/containers/systemd
cp examples/miniflare/rusty-golf-miniflare.container ~/.config/containers/systemd/

systemctl --user daemon-reload
systemctl --user start rusty-golf-miniflare
```

## Configure .env tests
Add these to the repo root `.env`:

```text
touch .env
echo "MINIFLARE_URL=http://127.0.0.1:8787" >> .env
token="$(cat miniflare_token.txt)"
echo "MINIFLARE_ADMIN_TOKEN=$token" >> .env
rm miniflare_token.txt
```

Then run the tests:
```bash
cargo nextest run --no-fail-fast
```

## Notes
- Re-run `make -C examples/miniflare` if you change `core/`, `serverless/`, or `static/` and want the container to pick up those updates.
- If you want the service to start automatically, enable it with `systemctl --user enable rusty-golf-miniflare`.
