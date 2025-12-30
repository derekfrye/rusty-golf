#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-8787}"
CONFIG_PATH="${2:-rusty-golf-serverless/wrangler.toml}"
config_dir="$(cd -- "$(dirname -- "${CONFIG_PATH}")" && pwd)"
workspace_root="$(cd -- "${config_dir}/.." && pwd)"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
serverless_dir="${script_dir}/.."
log_dir="${WRANGLER_LOG_DIR:-${workspace_root}/.wrangler-logs}"
config_dir="${XDG_CONFIG_HOME:-${workspace_root}/.wrangler-config}"
lock_dir="${WRANGLER_BUILD_LOCK_DIR:-${workspace_root}/.wrangler-locks}"
wrangler_env="${WRANGLER_ENV:-dev}"
real_worker_build="$(command -v worker-build)"

mkdir -p "${log_dir}"
mkdir -p "${config_dir}"
mkdir -p "${lock_dir}"
export WRANGLER_LOG_DIR="${log_dir}"
export XDG_CONFIG_HOME="${config_dir}"
export WRANGLER_BUILD_LOCK_DIR="${lock_dir}"
export WORKER_BUILD_BIN="${real_worker_build}"
# add our script dir ahead of path, where we have a flock'd version of worker-build
export PATH="${script_dir}:${PATH}"
cd "${workspace_root}"
exec wrangler dev --local --port "${PORT}" --ip 127.0.0.1 \
  --config "${CONFIG_PATH}" --env "${wrangler_env}"
