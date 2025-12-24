#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-8787}"
CONFIG_PATH="${2:-rusty-golf-serverless/wrangler.toml}"

exec wrangler dev --local --port "${PORT}" --ip 127.0.0.1 --config "${CONFIG_PATH}"
